//! Line locator grammar for tree-sitter-compatible query captures.

#[derive(Clone)]
pub(super) struct SyntaxQuerySelector {
    pub(super) path: String,
    pub(super) start_line: Option<usize>,
    pub(super) end_line: Option<usize>,
    pub(super) matches_all_paths: bool,
}

impl SyntaxQuerySelector {
    pub(super) fn display(&self) -> String {
        match (self.start_line, self.end_line) {
            (Some(start), Some(end)) => syntax_line_locator(&self.path, start, end),
            _ => self.path.clone(),
        }
    }

    pub(super) fn path(&self) -> &str {
        &self.path
    }
}

pub(super) fn syntax_line_locator(path: &str, start_line: usize, end_line: usize) -> String {
    let start = start_line.max(1);
    let end = end_line.max(start);
    if start == end {
        format!("{path}:{start}")
    } else {
        format!("{path}:{start}:{end}")
    }
}

pub(super) fn parse_syntax_query_selector(value: &str) -> Result<SyntaxQuerySelector, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("query --selector value cannot be empty".to_string());
    }
    if let Some(selector) = parse_hash_line_selector(trimmed)? {
        return Ok(selector);
    }
    let parts = trimmed.rsplitn(3, ':').collect::<Vec<_>>();
    if parts.len() == 3 {
        let end = parse_selector_line(parts[0], "end")?;
        let start = parse_selector_line(parts[1], "start")?;
        let path = parts[2].to_string();
        if path.is_empty() {
            return Err(format!("query --selector path is empty: {value}"));
        }
        return Ok(SyntaxQuerySelector {
            path,
            start_line: Some(start),
            end_line: Some(end.max(start)),
            matches_all_paths: false,
        });
    }
    if let Some((path, line)) = trimmed.rsplit_once(':')
        && path.ends_with(".rs")
        && !path.is_empty()
    {
        let line = parse_selector_line(line, "line")?;
        return Ok(SyntaxQuerySelector {
            path: path.to_string(),
            start_line: Some(line),
            end_line: Some(line),
            matches_all_paths: false,
        });
    }
    Ok(SyntaxQuerySelector {
        path: trimmed.to_string(),
        start_line: None,
        end_line: None,
        matches_all_paths: false,
    })
}

pub(super) fn syntax_selector_matches(
    selector: Option<&SyntaxQuerySelector>,
    path: &str,
    capture_start: usize,
    capture_end: usize,
    item_start: usize,
    item_end: usize,
) -> bool {
    let Some(selector) = selector else {
        return true;
    };
    if !selector.matches_all_paths && !selector_path_matches(&selector.path, path) {
        return false;
    }
    match (selector.start_line, selector.end_line) {
        (Some(start), Some(end)) => {
            line_ranges_overlap(start, end, capture_start, capture_end)
                || line_ranges_overlap(start, end, item_start, item_end)
        }
        _ => true,
    }
}

fn parse_hash_line_selector(value: &str) -> Result<Option<SyntaxQuerySelector>, String> {
    let Some((path, range)) = value.rsplit_once("#L") else {
        return Ok(None);
    };
    if path.is_empty() {
        return Err(format!("query --selector path is empty: {value}"));
    }
    let (start, end) = match range.split_once('-') {
        Some((start, end)) => (
            parse_selector_line(start, "start")?,
            parse_selector_line(end.strip_prefix('L').unwrap_or(end), "end")?,
        ),
        None => {
            let line = parse_selector_line(range, "line")?;
            (line, line)
        }
    };
    Ok(Some(SyntaxQuerySelector {
        path: path.to_string(),
        start_line: Some(start),
        end_line: Some(end.max(start)),
        matches_all_paths: false,
    }))
}

fn parse_selector_line(value: &str, label: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("invalid query --selector {label} line `{value}`: {error}"))
        .map(|line| line.max(1))
}

fn selector_path_matches(selector_path: &str, row_path: &str) -> bool {
    normalize_selector_path(selector_path) == normalize_selector_path(row_path)
}

fn normalize_selector_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

fn line_ranges_overlap(
    left_start: usize,
    left_end: usize,
    right_start: usize,
    right_end: usize,
) -> bool {
    left_start <= right_end && right_start <= left_end
}
