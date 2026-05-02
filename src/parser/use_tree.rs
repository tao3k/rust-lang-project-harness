//! Native Rust `use` tree facts.

use syn::spanned::Spanned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustUseStatementSyntax {
    pub line: usize,
    pub visibility: RustUseVisibilityKind,
    pub context: RustUseStatementContext,
    pub imports: Vec<RustUseImportSyntax>,
    pub reexports: Vec<RustUseReexportSyntax>,
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
pub(crate) struct RustUseReexportSyntax {
    pub line: usize,
    pub source_segments: Vec<String>,
    pub exposed_name: String,
    pub visibility: RustUseVisibilityKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustUseImportSyntax {
    pub segments: Vec<String>,
    pub exposed_name: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RustUseVisibilityKind {
    Private,
    Public,
    Crate,
    Super,
    SelfScope,
    Restricted(Vec<String>),
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

impl RustUseReexportSyntax {
    #[cfg(test)]
    pub(crate) fn rendered_source_path(&self) -> String {
        self.source_segments.join("::")
    }
}

impl RustUseVisibilityKind {
    pub(crate) fn is_reexport(&self) -> bool {
        !matches!(self, Self::Private)
    }
}

pub(crate) fn rust_use_statement_syntax(
    item_use: &syn::ItemUse,
    context: RustUseStatementContext,
) -> RustUseStatementSyntax {
    let imports = use_item_imports(item_use);
    let visibility = use_visibility_kind(&item_use.vis);
    let reexports = if visibility.is_reexport() {
        imports
            .iter()
            .filter_map(|import| {
                reexport_syntax(item_use.span().start().line.max(1), &visibility, import)
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
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
        visibility,
        context,
        imports,
        reexports,
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
            let name = name.ident.to_string();
            push_named_import(prefix, name.clone(), name, is_absolute, imports);
        }
        syn::UseTree::Rename(rename) => {
            push_named_import(
                prefix,
                rename.ident.to_string(),
                rename.rename.to_string(),
                is_absolute,
                imports,
            );
        }
        syn::UseTree::Glob(_) => {
            imports.push(import_syntax(prefix.clone(), None, is_absolute, true));
        }
    }
}

fn push_named_import(
    prefix: &[String],
    ident: String,
    exposed_name: String,
    is_absolute: bool,
    imports: &mut Vec<RustUseImportSyntax>,
) {
    let mut segments = prefix.to_vec();
    segments.push(ident);
    imports.push(import_syntax(
        segments,
        Some(exposed_name),
        is_absolute,
        false,
    ));
}

fn import_syntax(
    segments: Vec<String>,
    exposed_name: Option<String>,
    is_absolute: bool,
    is_glob: bool,
) -> RustUseImportSyntax {
    let root_kind = import_root_kind(is_absolute, &segments);
    RustUseImportSyntax {
        parent_hops: parent_hops(&segments),
        is_prelude_import: segments.iter().any(|segment| segment == "prelude"),
        segments,
        exposed_name,
        is_absolute,
        root_kind,
        is_glob,
    }
}

fn reexport_syntax(
    line: usize,
    visibility: &RustUseVisibilityKind,
    import: &RustUseImportSyntax,
) -> Option<RustUseReexportSyntax> {
    Some(RustUseReexportSyntax {
        line,
        source_segments: import.segments.clone(),
        exposed_name: import.exposed_name.clone()?,
        visibility: visibility.clone(),
    })
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

fn use_visibility_kind(visibility: &syn::Visibility) -> RustUseVisibilityKind {
    match visibility {
        syn::Visibility::Inherited => RustUseVisibilityKind::Private,
        syn::Visibility::Public(_) => RustUseVisibilityKind::Public,
        syn::Visibility::Restricted(restricted) => restricted_visibility_kind(restricted),
    }
}

fn restricted_visibility_kind(restricted: &syn::VisRestricted) -> RustUseVisibilityKind {
    let segments = restricted
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    match segments.as_slice() {
        [segment] if segment == "crate" => RustUseVisibilityKind::Crate,
        [segment] if segment == "super" => RustUseVisibilityKind::Super,
        [segment] if segment == "self" => RustUseVisibilityKind::SelfScope,
        [] => RustUseVisibilityKind::Unknown,
        _ => RustUseVisibilityKind::Restricted(segments),
    }
}
