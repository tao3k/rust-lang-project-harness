//! Rust native call-expression projection.

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use super::core::{
    MAX_SYNTAX_QUERY_ROWS, ProjectedItemsContext, capture_text_for_projection, compact_query_atom,
    compact_query_code, first_code_line_with_number, item_source_code, query_terms_match,
    syntax_query_predicates_match,
};
use crate::cli::tree_sitter_query_locator::syntax_selector_matches;
use crate::cli::tree_sitter_query_packet::SyntaxQueryRow;

pub(super) fn collect_projected_calls(item: &syn::Item, context: &mut ProjectedItemsContext<'_>) {
    match item {
        syn::Item::Fn(function) => {
            let span = item.span();
            let start_line = span.start().line.max(1);
            let end_line = span.end().line.max(start_line);
            let owner = CallProjectionOwner {
                start_line,
                end_line,
                code: item_source_code(context.source_lines, start_line, end_line),
            };
            let mut visitor = CallProjectionVisitor { context, owner };
            visitor.visit_block(&function.block);
        }
        syn::Item::Impl(item_impl) => {
            for impl_item in &item_impl.items {
                let syn::ImplItem::Fn(function) = impl_item else {
                    continue;
                };
                let span = function.span();
                let start_line = span.start().line.max(1);
                let end_line = span.end().line.max(start_line);
                let owner = CallProjectionOwner {
                    start_line,
                    end_line,
                    code: item_source_code(context.source_lines, start_line, end_line),
                };
                let mut visitor = CallProjectionVisitor { context, owner };
                visitor.visit_block(&function.block);
            }
        }
        syn::Item::Trait(item_trait) => {
            for trait_item in &item_trait.items {
                let syn::TraitItem::Fn(function) = trait_item else {
                    continue;
                };
                let Some(block) = &function.default else {
                    continue;
                };
                let span = function.span();
                let start_line = span.start().line.max(1);
                let end_line = span.end().line.max(start_line);
                let owner = CallProjectionOwner {
                    start_line,
                    end_line,
                    code: item_source_code(context.source_lines, start_line, end_line),
                };
                let mut visitor = CallProjectionVisitor { context, owner };
                visitor.visit_block(block);
            }
        }
        _ => {}
    }
}

struct CallProjectionOwner {
    start_line: usize,
    end_line: usize,
    code: String,
}

struct CallProjectionVisitor<'context, 'source> {
    context: &'context mut ProjectedItemsContext<'source>,
    owner: CallProjectionOwner,
}

impl Visit<'_> for CallProjectionVisitor<'_, '_> {
    fn visit_expr_call(&mut self, node: &syn::ExprCall) {
        self.project_call(node.span(), "call.target", call_target_name(&node.func));
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &syn::ExprMethodCall) {
        self.project_call(node.span(), "call.method", node.method.to_string());
        visit::visit_expr_method_call(self, node);
    }
}

impl CallProjectionVisitor<'_, '_> {
    fn project_call(&mut self, span: proc_macro2::Span, preferred_capture: &str, target: String) {
        let start_line = span.start().line.max(1);
        let end_line = span.end().line.max(start_line);
        let (code_line, code_source) =
            first_code_line_with_number(self.context.source_lines, start_line, end_line);
        let code = compact_query_code(code_source);
        let capture = capture_for_call(preferred_capture, self.context.captures);
        let name = compact_query_atom(&target);
        let capture_text = capture_text_for_projection(&capture, &name, &code, &self.owner.code);
        if !syntax_selector_matches(
            self.context.selector,
            self.context.relative_path,
            start_line,
            end_line,
            self.owner.start_line,
            self.owner.end_line,
        ) {
            return;
        }
        if !query_terms_match(&capture_text, self.context.terms) {
            return;
        }
        if !syntax_query_predicates_match(
            &capture,
            &capture_text,
            &name,
            &code,
            &self.owner.code,
            self.context.predicates,
        ) {
            return;
        }
        *self.context.total_matches += 1;
        if self.context.rows.len() < MAX_SYNTAX_QUERY_ROWS {
            self.context.rows.push(SyntaxQueryRow {
                capture,
                capture_text,
                node: "call_expression",
                name,
                path: self.context.relative_path.to_string(),
                start_line: code_line,
                end_line,
                item_start_line: self.owner.start_line,
                item_end_line: self.owner.end_line,
                item_code: self.owner.code.clone(),
            });
        }
    }
}

fn capture_for_call(preferred_capture: &str, captures: &[String]) -> String {
    captures
        .iter()
        .find(|capture| capture.as_str() == preferred_capture)
        .or_else(|| captures.iter().find(|capture| capture.starts_with("call.")))
        .or_else(|| captures.first())
        .cloned()
        .unwrap_or_else(|| "call.target".to_string())
}

fn call_target_name(function: &syn::Expr) -> String {
    match function {
        syn::Expr::Path(path) => path_query_name(&path.path),
        syn::Expr::Field(field) => member_query_name(&field.member),
        syn::Expr::MethodCall(method) => method.method.to_string(),
        syn::Expr::Paren(paren) => call_target_name(&paren.expr),
        syn::Expr::Reference(reference) => call_target_name(&reference.expr),
        _ => compact_query_atom(&function.to_token_stream().to_string()),
    }
}

fn path_query_name(path: &syn::Path) -> String {
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    if segments.is_empty() {
        "call".to_string()
    } else {
        segments.join("::")
    }
}

fn member_query_name(member: &syn::Member) -> String {
    match member {
        syn::Member::Named(ident) => ident.to_string(),
        syn::Member::Unnamed(index) => index.index.to_string(),
    }
}
