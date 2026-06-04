//! Hook-oriented query command mapped onto provider-owned search views.

use std::env;
use std::path::{Path, PathBuf};

use crate::parser::{RustTopLevelItemSyntax, parse_rust_file};
const MAX_EXACT_DIRECT_READ_LINES: usize = 40;
const READ_PLAN_FRONTIER_LIMIT: usize = 8;

pub(super) enum QueryCommand {
    Help,
    Search(QuerySearchOptions),
}

#[derive(Debug, Default)]
pub(super) struct QuerySearchOptions {
    pub(super) view: String,
    pub(super) query: Option<String>,
    pub(super) json: bool,
    pub(super) output_view: Option<String>,
    pub(super) package: Option<String>,
    pub(super) seeds: Option<usize>,
    pub(super) pipes: Vec<String>,
    pub(super) query_set: Vec<String>,
    pub(super) item_query: Option<String>,
    pub(super) read_selector: Option<String>,
    pub(super) item_names_only: bool,
    pub(super) item_code: bool,
    pub(super) paths: Vec<PathBuf>,
}

pub(super) fn parse_query(
    args: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<QueryCommand, String> {
    let options = QueryOptions::parse(args)?;
    if options.help {
        return Ok(QueryCommand::Help);
    }
    let wants_direct_source_items = options.from_hook.as_deref() == Some("direct-source-read")
        && options.query.is_none()
        && options.terms.is_empty()
        && options
            .selector
            .as_deref()
            .is_some_and(is_exact_direct_source_selector);
    let mut search_options = options.search_options()?;
    if wants_direct_source_items && !search_options.pipes.iter().any(|pipe| pipe == "items") {
        search_options.pipes.push("items".to_string());
    }
    if wants_direct_source_items && search_options.output_view.as_deref() != Some("read-packet") {
        search_options.output_view = None;
    }
    Ok(QueryCommand::Search(search_options))
}

fn is_exact_direct_source_selector(selector: &str) -> bool {
    let selector = selector.strip_prefix("owner:").unwrap_or(selector);
    let path = selector.split(':').next().unwrap_or(selector);
    !path.is_empty()
        && !path.contains('*')
        && !path.contains('?')
        && !path.contains('[')
        && !path.contains('{')
}

pub(super) fn print_query_help() {
    println!(
        "rs-harness query <owner-path[:start:end]> [items tests] [--query SYMBOL] [--names-only | --code] [PROJECT_ROOT]\n\
rs-harness query --from-hook direct-source-read --selector <path[:line-range]> --code [PROJECT_ROOT]\n\
rs-harness query --from-hook KIND --selector SELECTOR [--query SYMBOL | --term TERM] [--names-only | --code] [PROJECT_ROOT]\n\
rs-harness query --term TERM [--term TERM...] [--surface PIPE] [--view seeds] [PROJECT_ROOT]\n\n\
Maps hook-denied raw reads and broad searches into parser-owned search output.\n\
Concrete Rust owner selectors route to search owner items/tests; multi-term queries route to search fzf query-set.\n\
Glob or broad selectors without terms route to search prime --view seeds.\n\
Owner item queries emit |query status=hit|miss match=exact|fallback-contains|none.\n\
Use --code after selecting an owner/symbol or hook path/range to emit compact parser-owned code."
    );
}

#[derive(Debug, Default)]
struct QueryOptions {
    selector: Option<String>,
    query: Option<String>,
    terms: Vec<String>,
    surfaces: Vec<String>,
    from_hook: Option<String>,
    names_only: bool,
    code: bool,
    json: bool,
    help: bool,
    output_view: Option<String>,
    package: Option<String>,
    seeds: Option<usize>,
    paths: Vec<PathBuf>,
}

impl QueryOptions {
    fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut positional_only = false;
        let mut pending_option: Option<String> = None;
        let mut positionals = Vec::<std::ffi::OsString>::new();
        for arg in args {
            if let Some(option) = pending_option.take() {
                let Some(value) = arg.to_str() else {
                    return Err(format!("expected UTF-8 value after {option}"));
                };
                match option.as_str() {
                    "--selector" => options.set_selector(value)?,
                    "--query" => options.set_query(value)?,
                    "--term" => options.terms.extend(split_csv_values(value)),
                    "--surface" | "--pipe" => options.add_surfaces(value)?,
                    "--from-hook" => options.from_hook = Some(value.to_string()),
                    "--package" => options.package = Some(value.to_string()),
                    "--view" => options.output_view = Some(value.to_string()),
                    "--seeds" => options.seeds = Some(parse_usize_option(&option, value)?),
                    _ => unreachable!("known pending query option"),
                }
                continue;
            }
            if positional_only {
                positionals.push(arg);
                continue;
            }
            let Some(value) = arg.to_str() else {
                positionals.push(arg);
                continue;
            };
            match value {
                "--" => positional_only = true,
                "--json" => options.json = true,
                "--help" | "-h" => options.help = true,
                "--names-only" => options.names_only = true,
                "--code" => options.code = true,
                "--selector" | "--query" | "--term" | "--surface" | "--pipe" | "--from-hook"
                | "--package" | "--view" | "--seeds" => {
                    pending_option = Some(value.to_string());
                }
                value if is_search_pipe(value) => options.surfaces.push(value.to_string()),
                value if value.starts_with('-') => {
                    return Err(format!("unknown query option: {value}"));
                }
                _ => positionals.push(arg),
            }
        }
        if let Some(option) = pending_option {
            return Err(format!("expected value after {option}"));
        }
        if options.help {
            return Ok(options);
        }
        if options.names_only && options.code {
            return Err("query --names-only and --code cannot be combined".to_string());
        }
        options.apply_positionals(positionals)?;
        if options.paths.len() > 1 {
            return Err("expected at most one PROJECT_ROOT argument".to_string());
        }
        if let Some(view) = options.output_view.as_deref()
            && !matches!(view, "graph" | "hits" | "both" | "seeds" | "read-packet")
        {
            return Err(format!("unknown query --view mode: {view}"));
        }
        Ok(options)
    }

    fn apply_positionals(&mut self, positionals: Vec<std::ffi::OsString>) -> Result<(), String> {
        let mut values = positionals
            .into_iter()
            .map(|value| {
                value
                    .into_string()
                    .map_err(|_| "expected UTF-8 query arguments".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        if self.selector.is_none()
            && values
                .first()
                .is_some_and(|value| self.should_treat_as_selector(value))
        {
            self.selector = Some(values.remove(0));
        }
        for value in values {
            self.paths.push(PathBuf::from(value));
        }
        Ok(())
    }

    fn should_treat_as_selector(&self, value: &str) -> bool {
        if matches!(value, "." | "..") {
            return false;
        }
        if selector_has_glob(value) || has_rust_file_selector(value) || has_selector_prefix(value) {
            return true;
        }
        (self.query.is_some() || !self.terms.is_empty() || self.from_hook.is_some())
            && !PathBuf::from(value).is_dir()
    }

    fn set_selector(&mut self, selector: &str) -> Result<(), String> {
        if self
            .selector
            .as_deref()
            .is_some_and(|existing| existing != selector)
        {
            return Err("expected only one query selector".to_string());
        }
        self.selector = Some(selector.to_string());
        Ok(())
    }

    fn set_query(&mut self, query: &str) -> Result<(), String> {
        if self
            .query
            .as_deref()
            .is_some_and(|existing| existing != query)
        {
            return Err("expected only one query expression".to_string());
        }
        self.query = Some(query.to_string());
        Ok(())
    }

    fn add_surfaces(&mut self, surfaces: &str) -> Result<(), String> {
        for surface in split_csv_values(surfaces) {
            if !is_search_pipe(&surface) {
                return Err(format!("unknown query surface: {surface}"));
            }
            self.surfaces.push(surface);
        }
        Ok(())
    }

    fn search_options(&self) -> Result<QuerySearchOptions, String> {
        let mut options = QuerySearchOptions {
            json: self.json,
            output_view: self.output_view.clone(),
            package: self.package.clone(),
            seeds: self.seeds,
            item_names_only: self.names_only,
            item_code: self.code,
            ..QuerySearchOptions::default()
        };
        options.paths.push(self.project_root()?);
        if let Some(selector) = self.normalized_selector() {
            if selector_has_glob(&selector) {
                if !self.terms.is_empty() || self.query.is_some() {
                    self.apply_fzf_query(&mut options)?;
                } else {
                    options.view = "prime".to_string();
                    options
                        .output_view
                        .get_or_insert_with(|| "seeds".to_string());
                }
            } else {
                options.view = "owner".to_string();
                options.query = Some(selector);
                options.pipes = self.query_surfaces(&["items", "tests"]);
                options.item_query = self.item_query();
                if options.item_query.is_none() {
                    options
                        .output_view
                        .get_or_insert_with(|| "seeds".to_string());
                }
            }
        } else if !self.terms.is_empty() || self.query.is_some() {
            self.apply_fzf_query(&mut options)?;
        } else {
            options.view = "prime".to_string();
            options
                .output_view
                .get_or_insert_with(|| "seeds".to_string());
        }
        if self.from_hook.as_deref() == Some("direct-source-read") {
            options.read_selector = self.normalized_selector_preserving_range();
        }
        Ok(options)
    }

    fn apply_fzf_query(&self, options: &mut QuerySearchOptions) -> Result<(), String> {
        let terms = self.fzf_terms();
        if terms.is_empty() {
            return Err("query fzf mode requires --term or --query".to_string());
        }
        options.view = "fzf".to_string();
        options.query = Some(terms.join(","));
        options.query_set = terms;
        options.pipes = self.query_surfaces(&["tests"]);
        options
            .output_view
            .get_or_insert_with(|| "seeds".to_string());
        Ok(())
    }

    fn item_query(&self) -> Option<String> {
        self.query
            .clone()
            .or_else(|| (!self.terms.is_empty()).then(|| self.terms.join("|")))
    }

    fn fzf_terms(&self) -> Vec<String> {
        if !self.terms.is_empty() {
            return self.terms.clone();
        }
        self.query
            .as_deref()
            .map(split_csv_values)
            .unwrap_or_default()
    }

    fn normalized_selector(&self) -> Option<String> {
        self.selector
            .as_deref()
            .map(strip_selector_prefix)
            .map(strip_rust_line_suffix)
            .map(str::trim)
            .filter(|selector| !selector.is_empty())
            .map(ToOwned::to_owned)
    }

    fn normalized_selector_preserving_range(&self) -> Option<String> {
        self.selector
            .as_deref()
            .map(strip_selector_prefix)
            .map(str::trim)
            .filter(|selector| !selector.is_empty())
            .map(ToOwned::to_owned)
    }

    fn query_surfaces(&self, defaults: &[&str]) -> Vec<String> {
        if self.surfaces.is_empty() {
            defaults
                .iter()
                .map(|surface| (*surface).to_string())
                .collect()
        } else {
            self.surfaces.clone()
        }
    }

    fn project_root(&self) -> Result<PathBuf, String> {
        match self.paths.as_slice() {
            [path] => Ok(path.clone()),
            [] => {
                env::current_dir().map_err(|error| format!("failed to read current dir: {error}"))
            }
            _ => unreachable!("parse enforces at most one path"),
        }
    }
}

fn selector_has_glob(value: &str) -> bool {
    value
        .chars()
        .any(|character| matches!(character, '*' | '?' | '[' | ']' | '{' | '}'))
}

fn has_selector_prefix(value: &str) -> bool {
    value
        .split_once(':')
        .is_some_and(|(prefix, _)| matches!(prefix, "owner" | "tests" | "test" | "path"))
}

fn has_rust_file_selector(value: &str) -> bool {
    strip_rust_line_suffix(strip_selector_prefix(value)).ends_with(".rs")
}

fn strip_selector_prefix(value: &str) -> &str {
    value
        .split_once(':')
        .and_then(|(prefix, rest)| {
            matches!(prefix, "owner" | "tests" | "test" | "path").then_some(rest)
        })
        .unwrap_or(value)
}

fn strip_rust_line_suffix(value: &str) -> &str {
    let Some((path_and_start, end_line)) = value.rsplit_once(':') else {
        return value;
    };
    let Some((path, start_line)) = path_and_start.rsplit_once(':') else {
        return value;
    };
    if !path.ends_with(".rs") {
        return value;
    }
    if start_line.parse::<usize>().is_ok() && end_line.parse::<usize>().is_ok() {
        path
    } else {
        value
    }
}

pub(super) fn render_query_local_window(
    project_root: &Path,
    selector: &str,
) -> Result<Option<String>, String> {
    let Some((path, start_line, end_line)) = parse_query_local_window_selector(selector) else {
        return Ok(None);
    };
    let source_path = query_local_window_source_path(project_root, path);
    if query_local_window_line_count(start_line, end_line) > MAX_EXACT_DIRECT_READ_LINES {
        if let Some(rendered) = render_query_local_window_symbol_read_plan(
            path,
            &source_path,
            start_line,
            end_line,
            "wide-selector",
            "unknown",
        ) {
            return Ok(Some(rendered));
        }
        return Ok(Some(render_query_local_window_read_plan(
            path,
            start_line,
            end_line,
            start_line,
            end_line,
            "wide-selector",
            "unknown",
        )));
    }
    let source = std::fs::read_to_string(&source_path)
        .map_err(|error| format!("failed to read query local window {path}: {error}"))?;
    if let Some(rendered) =
        render_query_local_window_items(&source_path, &source, start_line, end_line)
    {
        return Ok(Some(rendered));
    }
    let line_count = end_line.saturating_sub(start_line).saturating_add(1);
    let mut rendered = source
        .lines()
        .skip(start_line.saturating_sub(1))
        .take(line_count)
        .map(compact_query_local_window_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if !rendered.is_empty() && is_low_signal_query_local_window(&rendered) {
        return Ok(Some(render_query_local_window_read_plan(
            path,
            start_line,
            end_line,
            start_line,
            end_line,
            "low-signal-window",
            "low",
        )));
    }
    if !rendered.is_empty() {
        rendered.push('\n');
    }
    Ok(Some(rendered))
}

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
        append_query_local_window_item_rows(&mut rows, item, start_line, end_line);
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
        rows.push((node.depth, label));
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

fn query_local_window_source_path(project_root: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    let rooted = project_root.join(path);
    if rooted.is_file() {
        rooted
    } else {
        path.to_path_buf()
    }
}

fn compact_query_local_window_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn query_local_window_line_count(start_line: usize, end_line: usize) -> usize {
    end_line.saturating_sub(start_line).saturating_add(1)
}

fn is_low_signal_query_local_window(text: &str) -> bool {
    !text.chars().any(|ch| ch.is_alphanumeric() || ch == '_')
}

fn render_query_local_window_symbol_read_plan(
    path: &str,
    source_path: &Path,
    requested_start: usize,
    requested_end: usize,
    reason: &str,
    density: &str,
) -> Option<String> {
    let module = parse_rust_file(source_path);
    if !module.report.is_valid {
        return None;
    }
    let symbols = read_plan_symbols(
        path,
        &module.syntax_facts.top_level_items,
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
        .collect::<Vec<_>>()
        .join(",");
    let frontier = symbols
        .iter()
        .map(|symbol| format!("{}.code", symbol.alias))
        .collect::<Vec<_>>()
        .join(",");
    let symbol_lines = symbols
        .iter()
        .map(|symbol| {
            format!(
                "|symbol item={} kind={} lineRange={} read={} lineCount={} reason=parser-item",
                symbol.name, symbol.kind, symbol.line_range, symbol.read, symbol.line_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!(
        "[read-plan] q={path} selector={path}:{requested_start}:{requested_end} mode=range-frontier code=false reason={reason} maxWindow={MAX_EXACT_DIRECT_READ_LINES} alg=symbol-frontier symbol={}\nlegend: ID=kind:role(value)!next; edge SRC>{{DST:rel}}; frontier ID.next\nalias: graph:{{R=range,{alias_kinds}}}\nR=range:requested({path}@{requested_start}:{requested_end})!outline;{aliases}\nR>{{{edges}}}\nrank={rank}\nfrontier={frontier}\nomit=code\navoid=repeat-wide-read,manual-window-scan,raw-read\n|range path={path} requested={requested_start}:{requested_end} selected={requested_start}:{requested_end} matched={requested_start}:{requested_end} coverage=full density={density}\n{symbol_lines}\n",
        symbols.len()
    ))
}

struct ReadPlanSymbol {
    alias: String,
    name: String,
    kind: &'static str,
    line_range: String,
    read: String,
    line_count: usize,
}

fn read_plan_symbols(
    path: &str,
    items: &[RustTopLevelItemSyntax],
    start_line: usize,
    end_line: usize,
) -> Vec<ReadPlanSymbol> {
    items
        .iter()
        .filter(|item| item.end_line >= start_line && item.line <= end_line)
        .filter_map(|item| {
            let line_count = query_local_window_line_count(item.line, item.end_line);
            if line_count > MAX_EXACT_DIRECT_READ_LINES {
                return None;
            }
            let name = read_plan_item_name(item)?;
            let line_range = format!("{}:{}", item.line, item.end_line);
            Some((name, item.kind, line_range, line_count))
        })
        .take(READ_PLAN_FRONTIER_LIMIT)
        .enumerate()
        .map(|(index, (name, kind, line_range, line_count))| {
            let alias = if index == 0 {
                "S".to_string()
            } else {
                format!("S{}", index + 1)
            };
            ReadPlanSymbol {
                alias,
                name: read_plan_token(&name),
                kind,
                read: format!("{path}:{line_range}"),
                line_range,
                line_count,
            }
        })
        .collect()
}

fn read_plan_item_name(item: &RustTopLevelItemSyntax) -> Option<String> {
    item.name
        .clone()
        .or_else(|| {
            item.impl_target_name
                .as_ref()
                .map(|target| format!("impl_{target}"))
        })
        .or_else(|| item.function_name.clone())
        .or_else(|| item.macro_name.clone())
}

fn read_plan_token(value: &str) -> String {
    let token = value
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || matches!(ch, '_' | '-' | ':' | '<' | '>' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if token.is_empty() {
        "item".to_string()
    } else {
        token
    }
}

fn render_query_local_window_read_plan(
    path: &str,
    requested_start: usize,
    requested_end: usize,
    selected_start: usize,
    selected_end: usize,
    reason: &str,
    density: &str,
) -> String {
    return render_query_local_window_read_plan_frontier(
        path,
        requested_start,
        requested_end,
        selected_start,
        selected_end,
        reason,
        density,
    );
}

fn render_query_local_window_read_plan_frontier(
    path: &str,
    requested_start: usize,
    requested_end: usize,
    selected_start: usize,
    selected_end: usize,
    reason: &str,
    density: &str,
) -> String {
    let selected_range = format!("{selected_start}:{selected_end}");
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
    let window_lines = windows
        .iter()
        .map(|window| {
            format!(
                "|window path={path} lineRange={} read={} lineCount={} reason=split",
                window.line_range, window.read, window.line_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "[read-plan] q={path} selector={path}:{requested_start}:{requested_end} mode=range-frontier code=false reason={reason} maxWindow={MAX_EXACT_DIRECT_READ_LINES} alg=range-split window={}\nlegend: ID=kind:role(value)!next; edge SRC>{{DST:rel}}; frontier ID.next\nalias: graph:{{R=range,{alias_kinds}}}\nR=range:requested({path}@{requested_start}:{requested_end})!outline;{aliases}\nR>{{{edges}}}\nrank={rank}\nfrontier={frontier}\nomit=code\navoid=repeat-wide-read,manual-window-scan,raw-read\n|range path={path} requested={requested_start}:{requested_end} selected={selected_range} matched={selected_range} coverage=full density={density}\n{window_lines}\n",
        windows.len()
    )
}

struct ReadPlanWindow {
    alias: String,
    line_range: String,
    read: String,
    line_count: usize,
}

fn read_plan_windows(path: &str, start_line: usize, end_line: usize) -> Vec<ReadPlanWindow> {
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
        windows.push(ReadPlanWindow {
            alias,
            read: format!("{path}:{line_range}"),
            line_count: query_local_window_line_count(start, end),
            line_range,
        });
        start = end.saturating_add(1);
    }
    windows
}

fn is_search_pipe(value: &str) -> bool {
    matches!(
        value,
        "owners"
            | "usage"
            | "items"
            | "tests"
            | "examples"
            | "benches"
            | "docs"
            | "docs-use"
            | "api"
            | "public-external-types"
            | "public-api"
            | "cfg"
            | "features"
            | "dependents"
    )
}

fn parse_usize_option(option: &str, value: &str) -> Result<usize, String> {
    value
        .parse()
        .map_err(|_| format!("expected integer value after {option}"))
}

fn split_csv_values(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
