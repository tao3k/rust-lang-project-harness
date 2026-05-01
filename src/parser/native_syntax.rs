//! Native Rust syntax facts shared by harness policies.

use std::path::{Path, PathBuf};

use proc_macro2::{TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use super::path_resolution::resolve_rust_path_attr;
use super::use_tree::{RustUseStatementContext, RustUseStatementSyntax, rust_use_statement_syntax};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RustNativeSyntaxFacts {
    pub has_module_doc: bool,
    pub top_level_items: Vec<RustTopLevelItemSyntax>,
    pub cfg_test_modules: Vec<RustModuleDeclarationSyntax>,
    pub test_function_count: usize,
    pub use_statements: Vec<RustUseStatementSyntax>,
    pub macro_invocations: Vec<RustInvocationSyntax>,
    pub function_calls: Vec<RustInvocationSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustTopLevelItemSyntax {
    pub line: usize,
    pub kind: &'static str,
    pub name: Option<String>,
    pub has_doc: bool,
    pub is_public: bool,
    pub is_public_use: bool,
    pub is_use: bool,
    pub is_extern_crate: bool,
    pub is_macro: bool,
    pub has_proc_macro_export_attr: bool,
    pub has_cfg_attr: bool,
    pub is_implementation_item: bool,
    pub function_name: Option<String>,
    pub macro_name: Option<String>,
    pub macro_declares_module: bool,
    pub macro_body_is_facade_boundary: bool,
    pub include_target: Option<String>,
    pub module: Option<RustModuleDeclarationSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustModuleDeclarationSyntax {
    pub line: usize,
    pub ident: String,
    pub path_attr: Option<String>,
    pub resolved_path_attr: Option<PathBuf>,
    pub is_inline: bool,
    pub is_cfg_test: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustInvocationSyntax {
    pub line: usize,
    pub terminal_name: String,
}

impl RustNativeSyntaxFacts {
    pub(crate) fn contains_macro_named(&self, names: &[&str]) -> bool {
        self.macro_invocations
            .iter()
            .any(|invocation| names.contains(&invocation.terminal_name.as_str()))
    }

    pub(crate) fn contains_function_call_named(&self, names: &[&str]) -> bool {
        self.function_calls
            .iter()
            .any(|invocation| names.contains(&invocation.terminal_name.as_str()))
    }

    pub(crate) fn contains_invocation_named(&self, names: &[&str]) -> bool {
        self.contains_macro_named(names) || self.contains_function_call_named(names)
    }
}

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

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(invocation) = invocation_syntax(&mac.path) {
            self.facts.macro_invocations.push(invocation);
        }
        visit::visit_macro(self, mac);
    }

    fn visit_expr_call(&mut self, expr_call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(expr_path) = expr_call.func.as_ref()
            && let Some(invocation) = invocation_syntax(&expr_path.path)
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
            self.module_stack.iter().any(|frame| frame.is_cfg_test),
        )
    }
}

fn top_level_item_syntax(item: &syn::Item, source_file: &Path) -> RustTopLevelItemSyntax {
    RustTopLevelItemSyntax {
        line: item.span().start().line.max(1),
        kind: item_kind(item),
        name: item_name(item),
        has_doc: item_attrs(item)
            .iter()
            .any(|attr| attr.path().is_ident("doc")),
        is_public: item_visibility(item).is_some_and(is_public_visibility),
        is_public_use: is_public_use(item),
        is_use: matches!(item, syn::Item::Use(_)),
        is_extern_crate: matches!(item, syn::Item::ExternCrate(_)),
        is_macro: matches!(item, syn::Item::Macro(_)),
        has_proc_macro_export_attr: item_attrs(item).iter().any(attribute_is_proc_macro_export),
        has_cfg_attr: item_attrs(item).iter().any(attribute_is_cfg),
        is_implementation_item: is_implementation_item(item),
        function_name: function_name_syntax(item),
        macro_name: macro_name_syntax(item),
        macro_declares_module: macro_declares_module_syntax(item),
        macro_body_is_facade_boundary: macro_body_is_facade_boundary_syntax(item),
        include_target: include_target_syntax(item),
        module: module_declaration_syntax(item, source_file),
    }
}

fn item_name(item: &syn::Item) -> Option<String> {
    match item {
        syn::Item::Const(item) => Some(item.ident.to_string()),
        syn::Item::Enum(item) => Some(item.ident.to_string()),
        syn::Item::ExternCrate(item) => Some(item.ident.to_string()),
        syn::Item::Fn(item) => Some(item.sig.ident.to_string()),
        syn::Item::Mod(item) => Some(item.ident.to_string()),
        syn::Item::Static(item) => Some(item.ident.to_string()),
        syn::Item::Struct(item) => Some(item.ident.to_string()),
        syn::Item::Trait(item) => Some(item.ident.to_string()),
        syn::Item::TraitAlias(item) => Some(item.ident.to_string()),
        syn::Item::Type(item) => Some(item.ident.to_string()),
        syn::Item::Union(item) => Some(item.ident.to_string()),
        _ => None,
    }
}

fn item_attrs(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(item) => &item.attrs,
        syn::Item::Enum(item) => &item.attrs,
        syn::Item::ExternCrate(item) => &item.attrs,
        syn::Item::Fn(item) => &item.attrs,
        syn::Item::Macro(item) => &item.attrs,
        syn::Item::Mod(item) => &item.attrs,
        syn::Item::Static(item) => &item.attrs,
        syn::Item::Struct(item) => &item.attrs,
        syn::Item::Trait(item) => &item.attrs,
        syn::Item::TraitAlias(item) => &item.attrs,
        syn::Item::Type(item) => &item.attrs,
        syn::Item::Union(item) => &item.attrs,
        _ => &[],
    }
}

fn function_name_syntax(item: &syn::Item) -> Option<String> {
    let syn::Item::Fn(item_fn) = item else {
        return None;
    };
    Some(item_fn.sig.ident.to_string())
}

fn macro_name_syntax(item: &syn::Item) -> Option<String> {
    let syn::Item::Macro(item_macro) = item else {
        return None;
    };
    invocation_syntax(&item_macro.mac.path).map(|invocation| invocation.terminal_name)
}

fn macro_declares_module_syntax(item: &syn::Item) -> bool {
    let syn::Item::Macro(item_macro) = item else {
        return false;
    };
    token_stream_declares_module(&item_macro.mac.tokens)
}

fn macro_body_is_facade_boundary_syntax(item: &syn::Item) -> bool {
    let syn::Item::Macro(item_macro) = item else {
        return false;
    };
    token_stream_is_facade_boundary(&item_macro.mac.tokens)
}

fn token_stream_is_facade_boundary(tokens: &TokenStream) -> bool {
    let Ok(file) = syn::parse2::<syn::File>(tokens.clone()) else {
        return false;
    };
    !file.items.is_empty() && file.items.iter().all(item_is_facade_boundary)
}

fn item_is_facade_boundary(item: &syn::Item) -> bool {
    match item {
        syn::Item::ExternCrate(_) | syn::Item::Use(_) => true,
        syn::Item::Mod(item_mod) => item_mod.content.is_none(),
        syn::Item::Macro(item_macro) => {
            invocation_syntax(&item_macro.mac.path)
                .is_some_and(|invocation| invocation.terminal_name != "macro_rules")
                && token_stream_is_facade_boundary(&item_macro.mac.tokens)
        }
        _ => false,
    }
}

fn token_stream_declares_module(tokens: &TokenStream) -> bool {
    let mut iter = tokens.clone().into_iter().peekable();
    while let Some(token) = iter.next() {
        match token {
            TokenTree::Group(group) if token_stream_declares_module(&group.stream()) => {
                return true;
            }
            TokenTree::Ident(ident)
                if ident == "mod"
                    && iter
                        .peek()
                        .is_some_and(|next| matches!(next, TokenTree::Ident(_))) =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

fn include_target_syntax(item: &syn::Item) -> Option<String> {
    let syn::Item::Macro(item_macro) = item else {
        return None;
    };
    include_literal_target(&item_macro.mac)
}

fn module_declaration_syntax(
    item: &syn::Item,
    source_file: &Path,
) -> Option<RustModuleDeclarationSyntax> {
    let syn::Item::Mod(item_mod) = item else {
        return None;
    };
    Some(module_declaration_from_item_mod(item_mod, source_file))
}

fn module_declaration_from_item_mod(
    item_mod: &syn::ItemMod,
    source_file: &Path,
) -> RustModuleDeclarationSyntax {
    let line = item_mod.attrs.first().map_or_else(
        || item_mod.span().start().line.max(1),
        |attr| attr.span().start().line.max(1),
    );
    let path_attr = path_attr_value(&item_mod.attrs);
    let resolved_path_attr = path_attr
        .as_deref()
        .map(|path_value| resolve_rust_path_attr(source_file, path_value));
    RustModuleDeclarationSyntax {
        line,
        ident: item_mod.ident.to_string(),
        path_attr,
        resolved_path_attr,
        is_inline: item_mod.content.is_some(),
        is_cfg_test: attrs_have_cfg_test(&item_mod.attrs),
    }
}

fn path_attr_value(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| {
        if !attr.path().is_ident("path") {
            return None;
        }
        let syn::Meta::NameValue(name_value) = &attr.meta else {
            return None;
        };
        let syn::Expr::Lit(expr_lit) = &name_value.value else {
            return None;
        };
        let syn::Lit::Str(lit_str) = &expr_lit.lit else {
            return None;
        };
        Some(lit_str.value())
    })
}

fn attrs_have_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(attribute_has_cfg_test)
}

fn attrs_have_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("test"))
}

fn attrs_have_doc(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("doc"))
}

fn attribute_is_proc_macro_export(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("proc_macro")
        || attr.path().is_ident("proc_macro_attribute")
        || attr.path().is_ident("proc_macro_derive")
}

fn attribute_is_cfg(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("cfg")
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

fn item_kind(item: &syn::Item) -> &'static str {
    match item {
        syn::Item::Const(_) => "const",
        syn::Item::Enum(_) => "enum",
        syn::Item::Fn(_) => "fn",
        syn::Item::Impl(_) => "impl",
        syn::Item::Macro(_) => "macro",
        syn::Item::Mod(_) => "mod",
        syn::Item::Static(_) => "static",
        syn::Item::Struct(_) => "struct",
        syn::Item::Trait(_) => "trait",
        syn::Item::TraitAlias(_) => "trait_alias",
        syn::Item::Type(_) => "type",
        syn::Item::Union(_) => "union",
        syn::Item::Use(_) => "use",
        _ => "item",
    }
}

fn item_visibility(item: &syn::Item) -> Option<&syn::Visibility> {
    match item {
        syn::Item::Const(item) => Some(&item.vis),
        syn::Item::Enum(item) => Some(&item.vis),
        syn::Item::Fn(item) => Some(&item.vis),
        syn::Item::Mod(item) => Some(&item.vis),
        syn::Item::Static(item) => Some(&item.vis),
        syn::Item::Struct(item) => Some(&item.vis),
        syn::Item::Trait(item) => Some(&item.vis),
        syn::Item::TraitAlias(item) => Some(&item.vis),
        syn::Item::Type(item) => Some(&item.vis),
        syn::Item::Union(item) => Some(&item.vis),
        _ => None,
    }
}

fn is_public_visibility(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

fn is_public_use(item: &syn::Item) -> bool {
    matches!(item, syn::Item::Use(item_use) if is_public_visibility(&item_use.vis))
}

fn is_implementation_item(item: &syn::Item) -> bool {
    matches!(
        item,
        syn::Item::Const(_)
            | syn::Item::Enum(_)
            | syn::Item::Fn(_)
            | syn::Item::Impl(_)
            | syn::Item::Static(_)
            | syn::Item::Struct(_)
            | syn::Item::Trait(_)
            | syn::Item::TraitAlias(_)
            | syn::Item::Type(_)
            | syn::Item::Union(_)
    )
}

fn invocation_syntax(path: &syn::Path) -> Option<RustInvocationSyntax> {
    let terminal_name = path.segments.last()?.ident.to_string();
    Some(RustInvocationSyntax {
        line: path.span().start().line.max(1),
        terminal_name,
    })
}

fn include_literal_target(mac: &syn::Macro) -> Option<String> {
    let invocation = invocation_syntax(&mac.path)?;
    if invocation.terminal_name != "include" {
        return None;
    }
    syn::parse2::<syn::LitStr>(mac.tokens.clone())
        .ok()
        .map(|lit| lit.value())
}
