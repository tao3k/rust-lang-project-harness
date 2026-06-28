//! Native Rust process command boundary facts.

use std::collections::BTreeMap;

use quote::ToTokens;
use syn::visit::{self, Visit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustProcessCommandExecutionSyntax {
    pub line: usize,
    pub function_line: usize,
    pub function_name: String,
    pub terminal_operation: String,
    pub command_expr: String,
    pub has_current_dir: bool,
    pub has_env: bool,
    pub is_test_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandBindingSyntax {
    line: usize,
    command_expr: String,
    has_current_dir: bool,
    has_env: bool,
}

pub(crate) fn process_command_execution_syntax(
    item: &syn::Item,
) -> Vec<RustProcessCommandExecutionSyntax> {
    match item {
        syn::Item::Fn(item_fn) => item_function_process_command_execution_syntax(item_fn, false),
        syn::Item::Impl(item_impl) => impl_process_command_execution_syntax(item_impl),
        _ => Vec::new(),
    }
}

fn attrs_have_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(attribute_has_cfg_test)
}

fn attribute_has_cfg_test(attr: &syn::Attribute) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    let syn::Meta::List(list) = &attr.meta else {
        return false;
    };
    list.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
        .map(|metas| {
            metas
                .iter()
                .any(|meta| matches!(meta, syn::Meta::Path(path) if path.is_ident("test")))
        })
        .unwrap_or(false)
}

fn item_function_process_command_execution_syntax(
    item_fn: &syn::ItemFn,
    inherited_test_context: bool,
) -> Vec<RustProcessCommandExecutionSyntax> {
    let mut visitor = ProcessCommandExecutionVisitor::new(
        item_fn.sig.ident.to_string(),
        item_fn.sig.ident.span().start().line.max(1),
        inherited_test_context || attrs_have_cfg_test(&item_fn.attrs),
    );
    visitor.visit_block(&item_fn.block);
    visitor.facts
}

fn impl_process_command_execution_syntax(
    item_impl: &syn::ItemImpl,
) -> Vec<RustProcessCommandExecutionSyntax> {
    let inherited_test_context = attrs_have_cfg_test(&item_impl.attrs);
    item_impl
        .items
        .iter()
        .flat_map(|item| {
            let syn::ImplItem::Fn(method) = item else {
                return Vec::new();
            };
            let mut visitor = ProcessCommandExecutionVisitor::new(
                method.sig.ident.to_string(),
                method.sig.ident.span().start().line.max(1),
                inherited_test_context || attrs_have_cfg_test(&method.attrs),
            );
            visitor.visit_block(&method.block);
            visitor.facts
        })
        .collect()
}

struct ProcessCommandExecutionVisitor {
    function_name: String,
    function_line: usize,
    is_test_context: bool,
    bindings: BTreeMap<String, CommandBindingSyntax>,
    facts: Vec<RustProcessCommandExecutionSyntax>,
}

impl ProcessCommandExecutionVisitor {
    fn new(function_name: String, function_line: usize, is_test_context: bool) -> Self {
        Self {
            function_name,
            function_line,
            is_test_context,
            bindings: BTreeMap::new(),
            facts: Vec::new(),
        }
    }

    fn push_fact(
        &mut self,
        line: usize,
        terminal_operation: String,
        command_expr: String,
        has_current_dir: bool,
        has_env: bool,
    ) {
        self.facts.push(RustProcessCommandExecutionSyntax {
            line,
            function_line: self.function_line,
            function_name: self.function_name.clone(),
            terminal_operation,
            command_expr,
            has_current_dir,
            has_env,
            is_test_context: self.is_test_context,
        });
    }
}

impl<'ast> Visit<'ast> for ProcessCommandExecutionVisitor {
    fn visit_local(&mut self, local: &'ast syn::Local) {
        if let syn::Pat::Ident(binding) = &local.pat
            && let Some(init) = &local.init
            && let Some(command_expr) = command_new_expression_text(&init.expr)
        {
            self.bindings.insert(
                binding.ident.to_string(),
                CommandBindingSyntax {
                    line: binding.ident.span().start().line.max(1),
                    command_expr,
                    has_current_dir: false,
                    has_env: false,
                },
            );
        }
        visit::visit_local(self, local);
    }

    fn visit_expr_method_call(&mut self, method_call: &'ast syn::ExprMethodCall) {
        let method_name = method_call.method.to_string();
        if let Some(binding_name) = receiver_root_ident(&method_call.receiver)
            && let Some(binding) = self.bindings.get_mut(&binding_name)
        {
            if method_name == "current_dir" {
                binding.has_current_dir = true;
            }
            if method_name == "env" || method_name == "envs" {
                binding.has_env = true;
            }
            if is_process_terminal_operation(&method_name) {
                let binding = binding.clone();
                self.push_fact(
                    method_call.method.span().start().line.max(binding.line),
                    method_name.clone(),
                    binding.command_expr,
                    binding.has_current_dir,
                    binding.has_env,
                );
            }
        }

        if is_process_terminal_operation(&method_name)
            && let Some(command_expr) = command_new_expression_text(&method_call.receiver)
        {
            self.push_fact(
                method_call.method.span().start().line.max(1),
                method_name,
                command_expr,
                receiver_chain_has_method(&method_call.receiver, "current_dir"),
                receiver_chain_has_any_method(&method_call.receiver, &["env", "envs"]),
            );
        }

        visit::visit_expr_method_call(self, method_call);
    }
}

fn is_process_terminal_operation(method_name: &str) -> bool {
    matches!(method_name, "output" | "status" | "spawn")
}

fn receiver_root_ident(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
            Some(path.path.segments.first()?.ident.to_string())
        }
        syn::Expr::Paren(paren) => receiver_root_ident(&paren.expr),
        syn::Expr::Reference(reference) => receiver_root_ident(&reference.expr),
        _ => None,
    }
}

fn command_new_expression_text(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Call(call) => {
            let syn::Expr::Path(path) = call.func.as_ref() else {
                return None;
            };
            is_command_new_path(&path.path).then(|| expr.to_token_stream().to_string())
        }
        syn::Expr::MethodCall(method_call) => command_new_expression_text(&method_call.receiver),
        syn::Expr::Paren(paren) => command_new_expression_text(&paren.expr),
        syn::Expr::Reference(reference) => command_new_expression_text(&reference.expr),
        _ => None,
    }
}

fn is_command_new_path(path: &syn::Path) -> bool {
    let mut segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    let Some(last) = segments.pop() else {
        return false;
    };
    let Some(previous) = segments.pop() else {
        return false;
    };
    last == "new" && previous == "Command"
}

fn receiver_chain_has_method(expr: &syn::Expr, method_name: &str) -> bool {
    match expr {
        syn::Expr::MethodCall(method_call) => {
            method_call.method == method_name
                || receiver_chain_has_method(&method_call.receiver, method_name)
        }
        syn::Expr::Paren(paren) => receiver_chain_has_method(&paren.expr, method_name),
        syn::Expr::Reference(reference) => receiver_chain_has_method(&reference.expr, method_name),
        _ => false,
    }
}

fn receiver_chain_has_any_method(expr: &syn::Expr, method_names: &[&str]) -> bool {
    method_names
        .iter()
        .any(|method_name| receiver_chain_has_method(expr, method_name))
}
