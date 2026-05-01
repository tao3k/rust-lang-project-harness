//! Native Rust `use` tree facts.

use syn::spanned::Spanned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustUseStatementSyntax {
    pub line: usize,
    pub context: RustUseStatementContext,
    pub imports: Vec<RustUseImportSyntax>,
    pub deep_relative_imports: Vec<RustUseDeepRelativeImportSyntax>,
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
    pub is_absolute: bool,
    pub scope_kind: RustUseGlobScopeKind,
    pub is_direct_parent_scope_glob: bool,
    pub is_parent_relative_glob: bool,
    pub is_prelude_glob: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustUseDeepRelativeImportSyntax {
    pub prefix_segments: Vec<String>,
    pub parent_hops: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustUseImportSyntax {
    pub segments: Vec<String>,
    pub is_absolute: bool,
    pub root_kind: RustUseImportRootKind,
    pub parent_hops: usize,
    pub is_glob: bool,
    pub is_prelude_import: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RustUseImportRootKind {
    Absolute,
    Crate,
    SelfScope,
    Parent,
    External,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RustUseGlobScopeKind {
    Absolute,
    CrateOwner,
    SelfScope,
    ParentScope,
    External,
    Unknown,
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
        let absolute_prefix = if self.is_absolute { "::" } else { "" };
        if self.prefix_segments.is_empty() {
            return format!("{absolute_prefix}*");
        }
        format!("{absolute_prefix}{}::*", self.prefix_segments.join("::"))
    }
}

impl RustUseDeepRelativeImportSyntax {
    pub(crate) fn rendered_path(&self) -> String {
        self.prefix_segments.join("::")
    }
}

pub(crate) fn rust_use_statement_syntax(
    item_use: &syn::ItemUse,
    context: RustUseStatementContext,
) -> RustUseStatementSyntax {
    let imports = use_item_imports(item_use);
    let deep_relative_imports = imports
        .iter()
        .filter(|import| is_deep_relative_import(import))
        .map(deep_relative_import_syntax)
        .collect::<Vec<_>>();
    let glob_imports = imports
        .iter()
        .filter(|import| import.is_glob)
        .map(glob_import_syntax)
        .collect::<Vec<_>>();
    RustUseStatementSyntax {
        line: item_use.span().start().line.max(1),
        context,
        imports,
        deep_relative_imports,
        contains_glob_import: !glob_imports.is_empty(),
        glob_imports,
    }
}

fn use_item_imports(item_use: &syn::ItemUse) -> Vec<RustUseImportSyntax> {
    let mut imports = Vec::new();
    let mut prefix = Vec::new();
    collect_use_tree_imports(
        &item_use.tree,
        item_use.leading_colon.is_some(),
        &mut prefix,
        &mut imports,
    );
    imports
}

fn collect_use_tree_imports(
    tree: &syn::UseTree,
    is_absolute: bool,
    prefix: &mut Vec<String>,
    imports: &mut Vec<RustUseImportSyntax>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_use_tree_imports(&path.tree, is_absolute, prefix, imports);
            prefix.pop();
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_use_tree_imports(item, is_absolute, prefix, imports);
            }
        }
        syn::UseTree::Name(name) => {
            push_named_import(prefix, name.ident.to_string(), is_absolute, imports);
        }
        syn::UseTree::Rename(rename) => {
            push_named_import(prefix, rename.ident.to_string(), is_absolute, imports);
        }
        syn::UseTree::Glob(_) => {
            imports.push(import_syntax(prefix.clone(), is_absolute, true));
        }
    }
}

fn push_named_import(
    prefix: &[String],
    ident: String,
    is_absolute: bool,
    imports: &mut Vec<RustUseImportSyntax>,
) {
    let mut segments = prefix.to_vec();
    segments.push(ident);
    imports.push(import_syntax(segments, is_absolute, false));
}

fn import_syntax(segments: Vec<String>, is_absolute: bool, is_glob: bool) -> RustUseImportSyntax {
    let root_kind = import_root_kind(is_absolute, &segments);
    RustUseImportSyntax {
        parent_hops: parent_hops(&segments),
        is_prelude_import: segments.iter().any(|segment| segment == "prelude"),
        segments,
        is_absolute,
        root_kind,
        is_glob,
    }
}

fn deep_relative_import_syntax(import: &RustUseImportSyntax) -> RustUseDeepRelativeImportSyntax {
    RustUseDeepRelativeImportSyntax {
        prefix_segments: import.segments.clone(),
        parent_hops: import.parent_hops,
    }
}

fn is_deep_relative_import(import: &RustUseImportSyntax) -> bool {
    import.root_kind == RustUseImportRootKind::Parent && import.parent_hops >= 2
}

fn glob_import_syntax(import: &RustUseImportSyntax) -> RustUseGlobImportSyntax {
    let is_direct_parent_scope_glob =
        matches!(import.segments.as_slice(), [segment] if segment == "super");
    let is_parent_relative_glob = import
        .segments
        .first()
        .is_some_and(|segment| segment == "super");
    let is_prelude_glob = import
        .segments
        .last()
        .is_some_and(|segment| segment == "prelude");
    RustUseGlobImportSyntax {
        prefix_segments: import.segments.clone(),
        is_absolute: import.is_absolute,
        scope_kind: glob_scope_kind(import.root_kind),
        is_direct_parent_scope_glob,
        is_parent_relative_glob,
        is_prelude_glob,
    }
}

fn glob_scope_kind(root_kind: RustUseImportRootKind) -> RustUseGlobScopeKind {
    match root_kind {
        RustUseImportRootKind::Absolute => RustUseGlobScopeKind::Absolute,
        RustUseImportRootKind::Crate => RustUseGlobScopeKind::CrateOwner,
        RustUseImportRootKind::SelfScope => RustUseGlobScopeKind::SelfScope,
        RustUseImportRootKind::Parent => RustUseGlobScopeKind::ParentScope,
        RustUseImportRootKind::External => RustUseGlobScopeKind::External,
        RustUseImportRootKind::Unknown => RustUseGlobScopeKind::Unknown,
    }
}

fn import_root_kind(is_absolute: bool, segments: &[String]) -> RustUseImportRootKind {
    if is_absolute {
        return RustUseImportRootKind::Absolute;
    }
    let Some(first_segment) = segments.first() else {
        return RustUseImportRootKind::Unknown;
    };
    match first_segment.as_str() {
        "crate" => RustUseImportRootKind::Crate,
        "self" => RustUseImportRootKind::SelfScope,
        "super" => RustUseImportRootKind::Parent,
        _ => RustUseImportRootKind::External,
    }
}

fn parent_hops(segments: &[String]) -> usize {
    segments
        .iter()
        .take_while(|segment| segment.as_str() == "super")
        .count()
}
