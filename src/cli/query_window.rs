use std::path::Path;

use super::query_source::{QuerySourceVersion, query_source_path, read_query_source_text};
use crate::parser::{RustTopLevelItemSyntax, parse_rust_file, syntax_abi::syntax_atom_for_kind};

const MAX_EXACT_DIRECT_READ_LINES: usize = 40;

pub(super) fn render_query_local_window(
    project_root: &Path,
    selector: &str,
    include_code: bool,
    source_version: QuerySourceVersion,
) -> Result<Option<String>, String> {
    let Some((path, start_line, end_line)) = parse_query_local_window_selector(selector) else {
        return Ok(None);
    };
    let source_path = query_source_path(project_root, path);
    if !include_code {
        return Ok(Some(render_query_local_window_read_plan(
            QueryLocalWindowReadPlan {
                path,
                source_path: &source_path,
                requested: QueryLocalWindowRange::new(start_line, end_line),
                selected: QueryLocalWindowRange::new(start_line, end_line),
                reason: "locator-frontier",
                density: "bounded",
            },
        )));
    }
    let source = read_query_source_text(project_root, path, &source_path, source_version)?;
    let line_count = query_local_window_line_count(start_line, end_line);
    let mut rendered = source
        .lines()
        .skip(start_line.saturating_sub(1))
        .take(line_count)
        .collect::<Vec<_>>()
        .join("\n");
    if !rendered.is_empty() && is_low_signal_query_local_window(&rendered) {
        return Ok(Some(render_query_local_window_read_plan(
            QueryLocalWindowReadPlan {
                path,
                source_path: &source_path,
                requested: QueryLocalWindowRange::new(start_line, end_line),
                selected: QueryLocalWindowRange::new(start_line, end_line),
                reason: "low-signal-window",
                density: "low",
            },
        )));
    }
    if !rendered.is_empty() {
        rendered.push('\n')
    }
    Ok(Some(rendered))
}

#[allow(dead_code)]
fn render_query_local_window_items(
    source_path: &Path,
    source: &str,
    start_line: usize,
    end_line: usize,
) -> Option<String> {
    if source.trim().is_empty() {
        return None;
    }
    let parsed = parse_rust_file(source_path);
    let mut rows = Vec::new();
    for item in parsed
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| query_lines_overlap(item.line, item.end_line, start_line, end_line))
    {
        append_query_local_window_item_rows(&mut rows, item, start_line, end_line)
    }
    let rendered =
        crate::parser::native_syntax::projection_code::compact_code_from_projection_nodes(
            &rows,
            |node| Some((node.0, node.1.clone())),
        );
    if rendered.trim().is_empty() {
        None
    } else {
        Some(format!("{rendered}\n"))
    }
}

#[allow(dead_code)]
fn append_query_local_window_item_rows(
    rows: &mut Vec<(usize, String)>,
    item: &RustTopLevelItemSyntax,
    start_line: usize,
    end_line: usize,
) {
    for node in &item.projection_nodes {
        if !query_lines_overlap(node.line, node.end_line, start_line, end_line) {
            continue;
        }
        let label = compact_query_local_window_line(&node.label);
        if label.is_empty() || rows.last().is_some_and(|(_, previous)| previous == &label) {
            continue;
        }
        rows.push((node.depth, label))
    }
}

fn query_lines_overlap(
    left_start: usize,
    left_end: usize,
    right_start: usize,
    right_end: usize,
) -> bool {
    left_start <= right_end && right_start <= left_end
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

#[allow(dead_code)]
fn compact_query_local_window_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn query_local_window_line_count(start_line: usize, end_line: usize) -> usize {
    end_line.saturating_sub(start_line).saturating_add(1)
}

fn is_low_signal_query_local_window(text: &str) -> bool {
    !text.chars().any(|ch| ch.is_alphanumeric() || ch == '_')
}

struct QueryLocalWindowRange {
    start: usize,
    end: usize,
}

impl QueryLocalWindowRange {
    fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

struct QueryLocalWindowReadPlan<'a> {
    path: &'a str,
    source_path: &'a Path,
    requested: QueryLocalWindowRange,
    selected: QueryLocalWindowRange,
    reason: &'a str,
    density: &'a str,
}

fn render_query_local_window_read_plan(plan: QueryLocalWindowReadPlan<'_>) -> String {
    if let Some(symbol_frontier) = render_query_local_window_symbol_read_plan(
        plan.path,
        plan.source_path,
        plan.requested.start,
        plan.requested.end,
    ) {
        return symbol_frontier;
    }
    render_query_local_window_range_read_plan(
        plan.path,
        plan.requested.start,
        plan.requested.end,
        plan.selected.start,
        plan.selected.end,
        plan.reason,
        plan.density,
    )
}

fn render_query_local_window_symbol_read_plan(
    path: &str,
    source_path: &Path,
    requested_start: usize,
    requested_end: usize,
) -> Option<String> {
    let parsed = parse_rust_file(source_path);
    let symbols = read_plan_symbols(
        &parsed.syntax_facts.top_level_items,
        path,
        requested_start,
        requested_end,
    );
    if symbols.is_empty() {
        return None;
    }
    let alias_kinds = symbols
        .iter()
        .map(|symbol| format!("{}=symbol", symbol.alias))
        .collect::<Vec<_>>()
        .join(",");
    let aliases = symbols
        .iter()
        .map(|symbol| {
            format!(
                "{}=symbol:{}({})@{}!code",
                symbol.alias, symbol.kind, symbol.name, symbol.read
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    let edges = symbols
        .iter()
        .map(|symbol| format!("{}:contains", symbol.alias))
        .collect::<Vec<_>>()
        .join(",");
    let rank = symbols
        .iter()
        .map(|symbol| symbol.alias.as_str())
        .chain(["R"])
        .collect::<Vec<_>>()
        .join(",");
    let frontier = symbols
        .iter()
        .map(|symbol| format!("{}.code", symbol.alias))
        .collect::<Vec<_>>()
        .join(",");
    let syn = symbols
        .first()
        .map(|symbol| symbol.syn)
        .unwrap_or("item/name");
    Some(format!(
        "[read-plan] q={path} selector={path}:{requested_start}:{requested_end} mode=range-frontier code=false reason=locator-frontier maxWindow={MAX_EXACT_DIRECT_READ_LINES} alg=symbol-frontier symbol={} syn={syn}\nlegend: ID=kind:role(value)!next; edge SRC>{{DST:rel}}; frontier ID.next\naliases: graph:{{R=range,{alias_kinds}}}\nR=range:requested({path}@{requested_start}:{requested_end})!outline;{aliases}\nR>{{{edges}}}\nrank={rank}\nfrontier={frontier}\nomit=code\navoid=repeat-wide-read,manual-window-scan,raw-read\n",
        symbols.len()
    ))
}

fn render_query_local_window_range_read_plan(
    path: &str,
    requested_start: usize,
    requested_end: usize,
    selected_start: usize,
    selected_end: usize,
    reason: &str,
    _density: &str,
) -> String {
    let windows = read_plan_windows(path, selected_start, selected_end);
    let alias_kinds = windows
        .iter()
        .map(|window| format!("{}=window", window.alias))
        .collect::<Vec<_>>()
        .join(",");
    let aliases = windows
        .iter()
        .map(|window| {
            format!(
                "{}=window:range({path}@{})!code",
                window.alias, window.line_range
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    let edges = windows
        .iter()
        .map(|window| format!("{}:split", window.alias))
        .collect::<Vec<_>>()
        .join(",");
    let rank = windows
        .iter()
        .map(|window| window.alias.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let frontier = windows
        .iter()
        .map(|window| format!("{}.code", window.alias))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "[read-plan] q={path} selector={path}:{requested_start}:{requested_end} mode=range-frontier code=false reason={reason} maxWindow={MAX_EXACT_DIRECT_READ_LINES} alg=range-split window={}\nlegend: ID=kind:role(value)!next; edge SRC>{{DST:rel}}; frontier ID.next\naliases: graph:{{R=range,{alias_kinds}}}\nR=range:requested({path}@{requested_start}:{requested_end})!outline;{aliases}\nR>{{{edges}}}\nrank={rank}\nfrontier={frontier}\nomit=code\navoid=repeat-wide-read,manual-window-scan,raw-read\n",
        windows.len()
    )
}

struct ReadPlanSymbol {
    alias: String,
    name: String,
    kind: &'static str,
    syn: &'static str,
    read: String,
}

fn read_plan_symbols(
    items: &[RustTopLevelItemSyntax],
    path: &str,
    start_line: usize,
    end_line: usize,
) -> Vec<ReadPlanSymbol> {
    items
        .iter()
        .filter(|item| query_lines_overlap(item.line, item.end_line, start_line, end_line))
        .filter_map(|item| read_plan_symbol(path, item))
        .enumerate()
        .map(|(index, mut symbol)| {
            symbol.alias = if index == 0 {
                "S".to_string()
            } else {
                format!("S{}", index + 1)
            };
            symbol
        })
        .collect()
}

fn read_plan_symbol(path: &str, item: &RustTopLevelItemSyntax) -> Option<ReadPlanSymbol> {
    let name = item
        .name
        .as_deref()
        .or(item.impl_target_name.as_deref())
        .or(item.function_name.as_deref())
        .or(item.macro_name.as_deref())?;
    Some(ReadPlanSymbol {
        alias: String::new(),
        name: name.to_string(),
        kind: item.kind,
        syn: syntax_atom_for_kind(item.kind),
        read: format!("{}:{}:{}", path, item.line, item.end_line),
    })
}

struct ReadPlanWindow {
    alias: String,
    line_range: String,
}

fn read_plan_windows(_path: &str, start_line: usize, end_line: usize) -> Vec<ReadPlanWindow> {
    let mut windows = Vec::new();
    let mut start = start_line;
    while start <= end_line {
        let end = end_line.min(start + MAX_EXACT_DIRECT_READ_LINES - 1);
        let line_range = format!("{start}:{end}");
        let alias = if windows.is_empty() {
            "W".to_string()
        } else {
            format!("W{}", windows.len() + 1)
        };
        windows.push(ReadPlanWindow { alias, line_range });
        start = end.saturating_add(1);
    }
    windows
}
