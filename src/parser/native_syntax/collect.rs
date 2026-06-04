//! Native Rust syntax collection.

use std::path::Path;

use syn::visit::{self, Visit};

use super::control_flow::function_control_flow_syntax;
use super::data_shape::{
    public_enum_tuple_variant_field_syntax, public_enum_variant_field_syntax,
    public_struct_field_syntax, public_type_alias_syntax, public_type_generic_bound_syntax,
};
use super::facts::RustNativeSyntaxFacts;
use super::invocation_facts::{function_call_invocation_syntax, macro_invocation_syntax};
use super::item_facts::top_level_item_syntax;
use super::module_facts::{
    attrs_have_cfg_test, attrs_have_doc, attrs_have_test, module_declaration_from_item_mod,
};
use super::path_facts::path_reference_syntax;
use super::signature::{
    public_function_param_syntax, public_function_return_syntax, public_function_tuple_api_syntax,
};
use crate::parser::{RustUseStatementContext, rust_use_statement_syntax};

pub(crate) fn rust_native_syntax_facts(
    syntax: &syn::File,
    source_file: &Path,
) -> RustNativeSyntaxFacts {
    let mut collector = NativeSyntaxCollector {
        source_file,
        facts: RustNativeSyntaxFacts::default(),
        module_stack: Vec::new(),
    };
    collector.visit_file(syntax);
    collector.facts.has_module_doc = attrs_have_doc(&syntax.attrs);
    collector.facts.top_level_items = syntax
        .items
        .iter()
        .map(|item| top_level_item_syntax(item, source_file))
        .collect();
    collector.facts.public_struct_fields = syntax
        .items
        .iter()
        .flat_map(public_struct_field_syntax)
        .collect();
    collector.facts.public_enum_variant_fields = syntax
        .items
        .iter()
        .flat_map(public_enum_variant_field_syntax)
        .collect();
    collector.facts.public_enum_tuple_variant_fields = syntax
        .items
        .iter()
        .flat_map(public_enum_tuple_variant_field_syntax)
        .collect();
    collector.facts.public_type_generic_bounds = syntax
        .items
        .iter()
        .flat_map(public_type_generic_bound_syntax)
        .collect();
    collector.facts.public_type_aliases = syntax
        .items
        .iter()
        .flat_map(public_type_alias_syntax)
        .collect();
    collector.facts.public_function_params = syntax
        .items
        .iter()
        .flat_map(public_function_param_syntax)
        .collect();
    collector.facts.public_function_returns = syntax
        .items
        .iter()
        .flat_map(public_function_return_syntax)
        .collect();
    collector.facts.public_tuple_api_surfaces = syntax
        .items
        .iter()
        .flat_map(public_function_tuple_api_syntax)
        .collect();
    collector.facts.all_function_control_flows = syntax
        .items
        .iter()
        .flat_map(function_control_flow_syntax)
        .collect();
    collector.facts.public_function_control_flows = collector
        .facts
        .all_function_control_flows
        .iter()
        .filter(|control_flow| control_flow.is_public)
        .cloned()
        .collect();
    collector.facts
}

struct NativeSyntaxCollector<'a> {
    source_file: &'a Path,
    facts: RustNativeSyntaxFacts,
    module_stack: Vec<RustModuleContextFrame>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustModuleContextFrame {
    ident: String,
    is_cfg_test: bool,
}

impl<'ast> Visit<'ast> for NativeSyntaxCollector<'_> {
    fn visit_item_mod(&mut self, item_mod: &'ast syn::ItemMod) {
        let is_cfg_test = attrs_have_cfg_test(&item_mod.attrs);
        if is_cfg_test {
            self.facts
                .cfg_test_modules
                .push(module_declaration_from_item_mod(item_mod, self.source_file));
        }
        self.module_stack.push(RustModuleContextFrame {
            ident: item_mod.ident.to_string(),
            is_cfg_test,
        });
        visit::visit_item_mod(self, item_mod);
        self.module_stack.pop();
    }

    fn visit_item_fn(&mut self, item_fn: &'ast syn::ItemFn) {
        if attrs_have_test(&item_fn.attrs) {
            self.facts.test_function_count += 1;
        }
        visit::visit_item_fn(self, item_fn);
    }

    fn visit_item_use(&mut self, item_use: &'ast syn::ItemUse) {
        self.facts
            .use_statements
            .push(rust_use_statement_syntax(item_use, self.use_context()));
        visit::visit_item_use(self, item_use);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        if let Some(reference) = path_reference_syntax(path, self.is_inside_cfg_test_module()) {
            self.facts.path_references.push(reference);
        }
        visit::visit_path(self, path);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(invocation) = macro_invocation_syntax(mac) {
            self.facts.macro_invocations.push(invocation);
        }
        visit::visit_macro(self, mac);
    }

    fn visit_expr_call(&mut self, expr_call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(expr_path) = expr_call.func.as_ref()
            && let Some(invocation) =
                function_call_invocation_syntax(&expr_path.path, expr_call.args.len())
        {
            self.facts.function_calls.push(invocation);
        }
        visit::visit_expr_call(self, expr_call);
    }
}

impl NativeSyntaxCollector<'_> {
    fn use_context(&self) -> RustUseStatementContext {
        RustUseStatementContext::from_enclosing_modules(
            self.module_stack
                .iter()
                .map(|frame| frame.ident.clone())
                .collect(),
            self.is_inside_cfg_test_module(),
        )
    }

    fn is_inside_cfg_test_module(&self) -> bool {
        self.module_stack.iter().any(|frame| frame.is_cfg_test)
    }
}
