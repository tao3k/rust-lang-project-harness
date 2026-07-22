use std::fmt::Write as _;
use std::path::Path;

use crate::parser::syntax_abi::{RUST_OWNER_ITEMS_QUERY_REF, syntax_atom_for_kind};
use crate::parser::{RustTopLevelItemSyntax, parse_rust_file};

use super::query_source::{QuerySourceVersion, query_source_path, read_query_source_text};

pub(super) fn render_query_local_item_frontier(
    project_root: &Path,
    selector: &str,
    item_query: &str,
    source_version: QuerySourceVersion,
    names_only: bool,
) -> Result<Option<String>, String> {
    if source_version != QuerySourceVersion::Worktree || item_query.contains('|') {
        return Ok(None);
    }
    let path = strip_query_local_window_selector_prefix(selector.trim());
    if path.is_empty() || parse_query_local_window_selector(path).is_some() {
        return Ok(None);
    }
    let source_path = query_source_path(project_root, path);
    if !source_path.is_file() {
        return Ok(None);
    }
    let parsed = parse_rust_file(&source_path);
    if item_query.trim().is_empty() {
        let source = read_query_source_text(project_root, path, &source_path, source_version)?;
        return Ok(Some(render_query_local_item_inventory(
            path,
            source.lines().count(),
            &parsed.syntax_facts.top_level_items,
            names_only,
        )));
    }
    let Some(item) = parsed
        .syntax_facts
        .top_level_items
        .iter()
        .find(|item| query_local_item_matches(item, item_query))
    else {
        return Ok(None);
    };
    let source = read_query_source_text(project_root, path, &source_path, source_version)?;
    let line_count = source.lines().count();
    let Some(item_name) = query_local_item_name(item) else {
        return Ok(None);
    };
    let output_field = if names_only { " output=names" } else { "" };
    Ok(Some(format!(
        "[query-item] q={path} pkg=. own=1 item=1 itemQuery={item_query}{output_field}\n\
|owner {path} role=source source=parser-visible-module lines={line_count} imports=0\n\
|query itemQuery={item_query} status=hit match=exact item=1 reason=parser-item-exact{output_field} next=query-code\n\
|item {item_name} kind={} next=syntax:{item_name} read={path}:{}:{} syn={} tsqRef={}\n",
        item.kind,
        item.line,
        item.end_line,
        syntax_atom_for_kind(item.kind),
        RUST_OWNER_ITEMS_QUERY_REF
    )))
}

fn render_query_local_item_inventory(
    path: &str,
    line_count: usize,
    items: &[RustTopLevelItemSyntax],
    names_only: bool,
) -> String {
    let named_items = items
        .iter()
        .filter_map(|item| query_local_item_name(item).map(|name| (item, name)))
        .collect::<Vec<_>>();
    let output_field = if names_only { " output=names" } else { "" };
    let mut rendered = format!(
        "[query-item] q={path} pkg=. selector=items alg=item-frontier{output_field}\n\
legend: ID=kind:role(value)!next; edge SRC>{{DST:rel}}; frontier ID.next\n\
aliases: graph:{{Q=query,O=owner,I=item}}\n\
O=owner:path({path})!owner"
    );
    for (index, (item, name)) in named_items.iter().enumerate() {
        let item_id = item_graph_id(index);
        let _ = write!(
            rendered,
            ";{item_id}=item:symbol({name})@{path}:{}:{}!syntax",
            item.line, item.end_line
        );
    }
    rendered.push('\n');
    let _ = writeln!(
        rendered,
        "|owner {path} role=source source=parser-visible-module lines={line_count} imports=0"
    );
    for (index, (item, name)) in named_items.iter().enumerate() {
        let item_id = item_graph_id(index);
        let _ = writeln!(
            rendered,
            "syntax {item_id} selector={path}:{}:{} syn={} tsqRef={}",
            item.line,
            item.end_line,
            syntax_atom_for_kind(item.kind),
            RUST_OWNER_ITEMS_QUERY_REF
        );
        let _ = writeln!(
            rendered,
            "|item {name} kind={} next=syntax:{name} read={path}:{}:{} syn={} tsqRef={}",
            item.kind,
            item.line,
            item.end_line,
            syntax_atom_for_kind(item.kind),
            RUST_OWNER_ITEMS_QUERY_REF
        );
    }
    if named_items.is_empty() {
        rendered.push_str("Q>{O:selects}\nrank=O frontier=O.owner\n");
    } else {
        rendered.push_str("Q>{O:selects}\nO>{");
        for index in 0..named_items.len() {
            if index > 0 {
                rendered.push(',');
            }
            let _ = write!(rendered, "{}:contains", item_graph_id(index));
        }
        rendered.push_str("}\nrank=");
        for index in 0..named_items.len() {
            if index > 0 {
                rendered.push(',');
            }
            rendered.push_str(&item_graph_id(index));
        }
        rendered.push_str(",O frontier=");
        for index in 0..named_items.len() {
            if index > 0 {
                rendered.push(',');
            }
            let _ = write!(rendered, "{}.syntax", item_graph_id(index));
        }
        rendered.push('\n');
    }
    rendered.push_str("omit=code,projection-nodes,large-item-text\n");
    rendered.push_str("avoid=inline-code-in-search,raw-read,repeat-owner\n");
    rendered
}

fn item_graph_id(index: usize) -> String {
    if index == 0 {
        "I".to_string()
    } else {
        format!("I{}", index + 1)
    }
}

fn parse_query_local_window_selector(selector: &str) -> Option<(&str, usize, usize)> {
    let selector = strip_query_local_window_selector_prefix(selector.trim());
    let (prefix, end) = selector.rsplit_once(':')?;
    if let Some((start, end)) = end.split_once('-') {
        return parse_query_local_window_range(prefix, start, end);
    }
    let (path, start) = prefix.rsplit_once(':')?;
    parse_query_local_window_range(path, start, end)
}

fn strip_query_local_window_selector_prefix(value: &str) -> &str {
    ["owner:", "read:", "path:"]
        .into_iter()
        .find_map(|prefix| value.strip_prefix(prefix))
        .unwrap_or(value)
}

fn parse_query_local_window_range<'a>(
    path: &'a str,
    start: &str,
    end: &str,
) -> Option<(&'a str, usize, usize)> {
    let start_line = start.parse::<usize>().ok()?;
    let end_line = end.parse::<usize>().ok()?;
    (start_line != 0 && end_line >= start_line).then_some((path, start_line, end_line))
}

fn query_local_item_matches(item: &RustTopLevelItemSyntax, item_query: &str) -> bool {
    query_local_item_name(item).is_some_and(|name| name == item_query)
}

fn query_local_item_name(item: &RustTopLevelItemSyntax) -> Option<&str> {
    item.name
        .as_deref()
        .or(item.impl_target_name.as_deref())
        .or(item.function_name.as_deref())
        .or(item.macro_name.as_deref())
}
