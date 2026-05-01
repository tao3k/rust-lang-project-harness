//! Native Rust `use` tree facts.

use syn::spanned::Spanned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustUseStatementSyntax {
    pub line: usize,
    pub context: RustUseStatementContext,
    pub contains_deep_relative_import: bool,
    pub contains_glob_import: bool,
    pub glob_imports: Vec<RustUseGlobImportSyntax>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RustUseStatementContext {
    pub enclosing_modules: Vec<String>,
    pub is_top_level: bool,
    pub is_inside_inline_module: bool,
    pub is_inside_cfg_test_module: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustUseGlobImportSyntax {
    pub prefix_segments: Vec<String>,
    pub is_direct_parent_scope_glob: bool,
    pub is_parent_relative_glob: bool,
    pub is_prelude_glob: bool,
}

impl RustUseStatementContext {
    pub(crate) fn from_enclosing_modules(
        enclosing_modules: Vec<String>,
        is_inside_cfg_test_module: bool,
    ) -> Self {
        let is_top_level = enclosing_modules.is_empty();
        Self {
            enclosing_modules,
            is_top_level,
            is_inside_inline_module: !is_top_level,
            is_inside_cfg_test_module,
        }
    }
}

impl RustUseGlobImportSyntax {
    pub(crate) fn rendered_path(&self) -> String {
        if self.prefix_segments.is_empty() {
            return "*".to_string();
        }
        format!("{}::*", self.prefix_segments.join("::"))
    }
}

pub(crate) fn rust_use_statement_syntax(
    item_use: &syn::ItemUse,
    context: RustUseStatementContext,
) -> RustUseStatementSyntax {
    let glob_imports = use_item_glob_imports(item_use);
    RustUseStatementSyntax {
        line: item_use.span().start().line.max(1),
        context,
        contains_deep_relative_import: use_item_contains_deep_relative_import(item_use),
        contains_glob_import: !glob_imports.is_empty(),
        glob_imports,
    }
}

fn use_item_contains_deep_relative_import(item_use: &syn::ItemUse) -> bool {
    let mut segments = Vec::new();
    use_tree_contains_super_super_with_prefix(&item_use.tree, &mut segments)
}

fn use_item_glob_imports(item_use: &syn::ItemUse) -> Vec<RustUseGlobImportSyntax> {
    let mut imports = Vec::new();
    let mut prefix = Vec::new();
    collect_use_tree_glob_imports(&item_use.tree, &mut prefix, &mut imports);
    imports
}

fn collect_use_tree_glob_imports(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    imports: &mut Vec<RustUseGlobImportSyntax>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_use_tree_glob_imports(&path.tree, prefix, imports);
            prefix.pop();
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_use_tree_glob_imports(item, prefix, imports);
            }
        }
        syn::UseTree::Glob(_) => imports.push(glob_import_syntax(prefix.clone())),
        syn::UseTree::Name(_) | syn::UseTree::Rename(_) => {}
    }
}

fn glob_import_syntax(prefix_segments: Vec<String>) -> RustUseGlobImportSyntax {
    let is_direct_parent_scope_glob =
        matches!(prefix_segments.as_slice(), [segment] if segment == "super");
    let is_parent_relative_glob = prefix_segments
        .first()
        .is_some_and(|segment| segment == "super");
    let is_prelude_glob = prefix_segments
        .last()
        .is_some_and(|segment| segment == "prelude");
    RustUseGlobImportSyntax {
        prefix_segments,
        is_direct_parent_scope_glob,
        is_parent_relative_glob,
        is_prelude_glob,
    }
}

fn use_tree_contains_super_super_with_prefix(
    tree: &syn::UseTree,
    segments: &mut Vec<String>,
) -> bool {
    match tree {
        syn::UseTree::Path(path) => {
            segments.push(path.ident.to_string());
            let contains = has_super_super(segments)
                || use_tree_contains_super_super_with_prefix(&path.tree, segments);
            segments.pop();
            contains
        }
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .any(|item| use_tree_contains_super_super_with_prefix(item, segments)),
        syn::UseTree::Name(name) => {
            segments.push(name.ident.to_string());
            let contains = has_super_super(segments);
            segments.pop();
            contains
        }
        syn::UseTree::Rename(rename) => {
            segments.push(rename.ident.to_string());
            let contains = has_super_super(segments);
            segments.pop();
            contains
        }
        syn::UseTree::Glob(_) => has_super_super(segments),
    }
}

fn has_super_super(segments: &[String]) -> bool {
    segments
        .windows(2)
        .any(|window| window[0] == "super" && window[1] == "super")
}
