//! Native Rust projection into tree-sitter-compatible query captures.

use std::fs;
use std::path::{Path, PathBuf};

use syn::spanned::Spanned;

use crate::parser::parse_rust_source_syntax;

use super::tree_sitter_query_locator::{SyntaxQuerySelector, syntax_selector_matches};
use super::tree_sitter_query_packet::SyntaxQueryRow;

const MAX_SYNTAX_QUERY_ROWS: usize = 80;
pub(super) const SUPPORTED_TREE_SITTER_QUERY_NODES: &[&str] = &[
    "const_item",
    "enum_item",
    "extern_crate_declaration",
    "function_item",
    "impl_item",
    "macro_definition",
    "macro_invocation",
    "mod_item",
    "static_item",
    "struct_item",
    "trait_item",
    "type_item",
    "use_declaration",
];

pub(super) struct SyntaxQueryProjection {
    pub(super) rows: Vec<SyntaxQueryRow>,
    pub(super) total_matches: usize,
    pub(super) truncated: bool,
    pub(super) unsupported_nodes: Vec<String>,
}

impl SyntaxQueryProjection {
    pub(super) fn match_status(&self) -> &'static str {
        if !self.unsupported_nodes.is_empty() && self.total_matches == 0 {
            "unsupported"
        } else if self.total_matches == 0 {
            "miss"
        } else {
            "hit"
        }
    }
}

pub(super) fn project_tree_sitter_query(
    project_root: &Path,
    query_node_types: &[String],
    captures: &[String],
    terms: &[String],
    selector: Option<&SyntaxQuerySelector>,
) -> Result<SyntaxQueryProjection, String> {
    let mut supported_nodes = query_node_types
        .iter()
        .filter_map(|node| {
            SUPPORTED_TREE_SITTER_QUERY_NODES
                .iter()
                .find(|supported| **supported == node)
                .copied()
        })
        .collect::<Vec<_>>();
    supported_nodes.sort_unstable();
    supported_nodes.dedup();
    let unsupported_nodes = if supported_nodes.is_empty() {
        query_node_types
            .iter()
            .filter(|node| !SUPPORTED_TREE_SITTER_QUERY_NODES.contains(&node.as_str()))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    if supported_nodes.is_empty() {
        return Ok(SyntaxQueryProjection {
            rows: Vec::new(),
            total_matches: 0,
            truncated: false,
            unsupported_nodes,
        });
    }

    let project_root = absolute_query_project_root(project_root);
    let source_files = rust_source_files(&project_root)?;
    let mut rows = Vec::new();
    let mut total_matches = 0usize;
    for path in source_files {
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(_) => continue,
        };
        let syntax = match parse_rust_source_syntax(&source) {
            Ok(syntax) => syntax,
            Err(_) => continue,
        };
        let relative_path = query_relative_path(&project_root, &path);
        let source_lines = source.lines().collect::<Vec<_>>();
        let mut context = ProjectedItemsContext {
            source_lines: &source_lines,
            relative_path: &relative_path,
            supported_nodes: &supported_nodes,
            terms,
            selector,
            captures,
            rows: &mut rows,
            total_matches: &mut total_matches,
        };
        collect_projected_items(&syntax.items, &mut context);
    }
    Ok(SyntaxQueryProjection {
        truncated: total_matches > rows.len(),
        rows,
        total_matches,
        unsupported_nodes,
    })
}

struct ProjectedItemsContext<'a> {
    source_lines: &'a [&'a str],
    relative_path: &'a str,
    supported_nodes: &'a [&'static str],
    terms: &'a [String],
    selector: Option<&'a SyntaxQuerySelector>,
    captures: &'a [String],
    rows: &'a mut Vec<SyntaxQueryRow>,
    total_matches: &'a mut usize,
}

fn collect_projected_items(items: &[syn::Item], context: &mut ProjectedItemsContext<'_>) {
    for item in items {
        if let Some(node) = tree_sitter_node_for_item(item, context.source_lines)
            && context.supported_nodes.contains(&node)
        {
            let span = item.span();
            let start_line = span.start().line.max(1);
            let end_line = span.end().line.max(start_line);
            let (code_line, code_source) =
                first_code_line_with_number(context.source_lines, start_line, end_line);
            let code = compact_query_code(code_source);
            if !query_terms_match(&code, context.terms) {
                continue;
            }
            if !syntax_selector_matches(
                context.selector,
                context.relative_path,
                code_line,
                code_line,
                start_line,
                end_line,
            ) {
                continue;
            }
            *context.total_matches += 1;
            if context.rows.len() < MAX_SYNTAX_QUERY_ROWS {
                let capture = capture_for_node(node, context.captures);
                let name = compact_query_atom(&item_query_name(item));
                let item_code = item_source_code(context.source_lines, start_line, end_line);
                let capture_text = capture_text_for_projection(&capture, &name, &code, &item_code);
                context.rows.push(SyntaxQueryRow {
                    capture,
                    capture_text,
                    node,
                    name,
                    path: context.relative_path.to_string(),
                    start_line: code_line,
                    end_line: code_line,
                    item_start_line: start_line,
                    item_end_line: end_line,
                    item_code,
                });
            }
        }
        if let syn::Item::Mod(module) = item
            && let Some((_, nested_items)) = &module.content
        {
            collect_projected_items(nested_items, context);
        }
    }
}

fn tree_sitter_node_for_item(item: &syn::Item, source_lines: &[&str]) -> Option<&'static str> {
    match item {
        syn::Item::Const(_) => Some("const_item"),
        syn::Item::Enum(_) => Some("enum_item"),
        syn::Item::ExternCrate(_) => Some("extern_crate_declaration"),
        syn::Item::Fn(_) => Some("function_item"),
        syn::Item::Impl(_) => Some("impl_item"),
        syn::Item::Macro(item) => {
            if first_code_line(
                source_lines,
                item.span().start().line,
                item.span().end().line,
            )
            .contains("macro_rules!")
            {
                Some("macro_definition")
            } else {
                Some("macro_invocation")
            }
        }
        syn::Item::Mod(_) => Some("mod_item"),
        syn::Item::Static(_) => Some("static_item"),
        syn::Item::Struct(_) => Some("struct_item"),
        syn::Item::Trait(_) | syn::Item::TraitAlias(_) => Some("trait_item"),
        syn::Item::Type(_) => Some("type_item"),
        syn::Item::Use(_) => Some("use_declaration"),
        _ => None,
    }
}

fn item_query_name(item: &syn::Item) -> String {
    match item {
        syn::Item::Const(item) => item.ident.to_string(),
        syn::Item::Enum(item) => item.ident.to_string(),
        syn::Item::ExternCrate(item) => item.ident.to_string(),
        syn::Item::Fn(item) => item.sig.ident.to_string(),
        syn::Item::Impl(item) => type_query_name(&item.self_ty),
        syn::Item::Macro(item) => item
            .ident
            .as_ref()
            .map(ToString::to_string)
            .or_else(|| {
                item.mac
                    .path
                    .segments
                    .last()
                    .map(|segment| segment.ident.to_string())
            })
            .unwrap_or_else(|| "macro".to_string()),
        syn::Item::Mod(item) => item.ident.to_string(),
        syn::Item::Static(item) => item.ident.to_string(),
        syn::Item::Struct(item) => item.ident.to_string(),
        syn::Item::Trait(item) => item.ident.to_string(),
        syn::Item::TraitAlias(item) => item.ident.to_string(),
        syn::Item::Type(item) => item.ident.to_string(),
        syn::Item::Use(_) => "use".to_string(),
        _ => "item".to_string(),
    }
}

fn type_query_name(ty: &syn::Type) -> String {
    if let syn::Type::Path(path) = ty
        && let Some(segment) = path.path.segments.last()
    {
        return segment.ident.to_string();
    }
    "impl".to_string()
}

fn capture_for_node(node: &str, captures: &[String]) -> String {
    let prefix = match node {
        "const_item" => "constant",
        "enum_item" => "enum",
        "extern_crate_declaration" => "extern",
        "function_item" => "function",
        "impl_item" => "impl",
        "macro_definition" | "macro_invocation" => "macro",
        "mod_item" => "module",
        "static_item" => "static",
        "struct_item" => "struct",
        "trait_item" => "trait",
        "type_item" => "type",
        "use_declaration" => "import",
        _ => "item",
    };
    captures
        .iter()
        .find(|capture| capture.starts_with(prefix))
        .or_else(|| {
            captures
                .iter()
                .find(|capture| capture.as_str() == "item.name")
        })
        .or_else(|| captures.first())
        .cloned()
        .unwrap_or_else(|| "item.name".to_string())
}

fn compact_query_atom(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("_")
}

fn capture_text_for_projection(
    capture: &str,
    name: &str,
    code_line: &str,
    item_code: &str,
) -> String {
    if capture.ends_with(".name") || capture.ends_with(".target") {
        name.to_string()
    } else if capture.ends_with(".definition") {
        item_code.to_string()
    } else {
        code_line.to_string()
    }
}

fn first_code_line<'a>(source_lines: &'a [&str], start_line: usize, end_line: usize) -> &'a str {
    first_code_line_with_number(source_lines, start_line, end_line).1
}

fn item_source_code(source_lines: &[&str], start_line: usize, end_line: usize) -> String {
    source_lines
        .iter()
        .skip(start_line.saturating_sub(1))
        .take(end_line.saturating_sub(start_line).saturating_add(1))
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
}

fn first_code_line_with_number<'a>(
    source_lines: &'a [&str],
    start_line: usize,
    end_line: usize,
) -> (usize, &'a str) {
    source_lines
        .iter()
        .skip(start_line.saturating_sub(1))
        .take(end_line.saturating_sub(start_line).saturating_add(1))
        .enumerate()
        .map(|(offset, line)| (start_line + offset, line.trim()))
        .find(|(_, line)| {
            !line.is_empty()
                && !line.starts_with("#[")
                && !line.starts_with("///")
                && !line.starts_with("//!")
        })
        .or_else(|| {
            source_lines
                .get(start_line.saturating_sub(1))
                .map(|line| (start_line, line.trim()))
        })
        .unwrap_or((start_line, ""))
}

fn query_terms_match(code: &str, terms: &[String]) -> bool {
    if terms.is_empty() {
        return true;
    }
    let code = code.to_ascii_lowercase();
    terms
        .iter()
        .map(|term| term.trim().to_ascii_lowercase())
        .filter(|term| !term.is_empty())
        .all(|term| code.contains(&term))
}

fn rust_source_files(project_root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for root in rust_query_source_roots(project_root) {
        collect_rust_source_files(&root, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn rust_query_source_roots(project_root: &Path) -> Vec<PathBuf> {
    if project_root.is_file() {
        return vec![project_root.to_path_buf()];
    }
    let mut roots = ["src", "tests", "benches", "examples"]
        .iter()
        .map(|name| project_root.join(name))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    let build_script = project_root.join("build.rs");
    if build_script.is_file() {
        roots.push(build_script);
    }
    if roots.is_empty() {
        roots.push(project_root.to_path_buf());
    }
    roots
}

fn collect_rust_source_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    if !path.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(path).map_err(|error| {
        format!(
            "failed to read Rust query project root {}: {error}",
            path.display()
        )
    })? {
        let entry = entry.map_err(|error| {
            format!(
                "failed to read Rust query project entry under {}: {error}",
                path.display()
            )
        })?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            if should_skip_query_dir(&entry_path) {
                continue;
            }
            collect_rust_source_files(&entry_path, files)?;
        } else if entry_path
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("rs")
        {
            files.push(entry_path);
        }
    }
    Ok(())
}

fn should_skip_query_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.starts_with('.')
        || matches!(
            name,
            "node_modules" | "target" | "vendor" | "dist" | "build" | "result"
        )
}

fn absolute_query_project_root(project_root: &Path) -> PathBuf {
    if project_root.is_absolute() {
        return project_root.to_path_buf();
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(project_root)
}

fn query_relative_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn compact_query_code(code: &str) -> String {
    const MAX_CODE_CHARS: usize = 200;
    let mut compact = code.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() > MAX_CODE_CHARS {
        compact = compact
            .chars()
            .take(MAX_CODE_CHARS.saturating_sub(3))
            .collect::<String>();
        compact.push_str("...");
    }
    compact
}
