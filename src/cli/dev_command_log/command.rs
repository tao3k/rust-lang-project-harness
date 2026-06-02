use std::collections::BTreeSet;

use super::constants::{SEARCH_PIPES, VALUE_OPTIONS};

#[derive(Clone)]
pub(crate) struct NormalizedCommand {
    pub(crate) namespace: String,
    pub(crate) method: String,
    pub(crate) pipes: Vec<String>,
    pub(crate) query: Option<String>,
    pub(crate) query_set_count: usize,
    pub(crate) render_mode: Option<String>,
    pub(crate) view: Option<String>,
}
pub(super) fn normalize_command(argv: &[String]) -> NormalizedCommand {
    let args = argv.iter().skip(1).cloned().collect::<Vec<_>>();
    let namespace_index = args.iter().position(|arg| !arg.starts_with('-'));
    let namespace = namespace_index
        .and_then(|index| args.get(index))
        .map(|arg| normalize_token(arg))
        .filter(|arg| !arg.is_empty())
        .unwrap_or_else(|| "cli".to_string());
    let render_mode = option_value(&args, "--view").map(|value| normalize_token(&value));
    let query_set_count = args
        .iter()
        .filter(|arg| arg.as_str() == "--query-set")
        .count()
        + args
            .iter()
            .filter(|arg| arg.starts_with("--query-set="))
            .count();
    let pipes = collect_pipes(&args);
    let view = if namespace == "search" {
        first_positional_after(&args, namespace_index.unwrap_or(0))
            .map(|value| normalize_token(&value))
    } else {
        None
    };
    let method = match namespace.as_str() {
        "search" => view
            .as_ref()
            .map(|view| format!("search/{view}"))
            .unwrap_or_else(|| "search".to_string()),
        "agent" => first_positional_after(&args, namespace_index.unwrap_or(0))
            .map(|subcommand| format!("agent/{}", normalize_token(&subcommand)))
            .unwrap_or_else(|| "agent".to_string()),
        other => other.to_string(),
    };
    let query = option_value(&args, "--query")
        .or_else(|| first_query_positional(&args, namespace_index.unwrap_or(0), view.as_deref()));

    NormalizedCommand {
        namespace,
        method,
        pipes,
        query,
        query_set_count,
        render_mode,
        view,
    }
}

pub(super) fn first_positional_after(args: &[String], start: usize) -> Option<String> {
    let mut skip_next = false;
    for (index, arg) in args.iter().enumerate().skip(start + 1) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if option_takes_value(arg) {
            skip_next = !arg.contains('=');
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        if index > start {
            return Some(arg.clone());
        }
    }
    None
}

pub(super) fn first_query_positional(
    args: &[String],
    namespace_index: usize,
    view: Option<&str>,
) -> Option<String> {
    let mut skip_next = false;
    let mut skipped_view = view.is_none();
    for arg in args.iter().skip(namespace_index + 1) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if option_takes_value(arg) {
            skip_next = !arg.contains('=');
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        let normalized = normalize_token(arg);
        if !skipped_view && Some(normalized.as_str()) == view {
            skipped_view = true;
            continue;
        }
        if SEARCH_PIPES.contains(&normalized.as_str()) {
            continue;
        }
        return Some(arg.clone());
    }
    None
}

pub(super) fn option_value(args: &[String], name: &str) -> Option<String> {
    for (index, arg) in args.iter().enumerate() {
        if arg == name {
            return args.get(index + 1).cloned();
        }
        if let Some(value) = arg.strip_prefix(&format!("{name}=")) {
            return Some(value.to_string());
        }
    }
    None
}

pub(super) fn option_takes_value(arg: &str) -> bool {
    let flag = arg.split_once('=').map(|(flag, _)| flag).unwrap_or(arg);
    VALUE_OPTIONS.contains(&flag)
}

pub(super) fn collect_pipes(args: &[String]) -> Vec<String> {
    let mut pipes = BTreeSet::new();
    for arg in args {
        let token = normalize_token(arg);
        if SEARCH_PIPES.contains(&token.as_str()) {
            pipes.insert(token);
        }
    }
    pipes.into_iter().collect()
}

pub(super) fn normalize_token(value: &str) -> String {
    let mut token = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            token.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            token.push(ch);
        }
    }
    if token.is_empty() {
        "unknown".to_string()
    } else {
        token
    }
}
