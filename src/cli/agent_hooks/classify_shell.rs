use super::classify_command::{
    command_candidate_paths, command_has_tool, is_rust_ingest_command, rust_source_path_or_glob,
    shell_tokens, source_root_scope,
};

pub(super) fn shell_bulk_reads_rust(command: &str) -> bool {
    if command.trim().is_empty() {
        return false;
    }
    let lower = command.replace('\\', "/").to_ascii_lowercase();
    if rg_searches_rust_without_ingest(command, &lower) {
        return true;
    }
    let reads_content = command_has_content_reader(&lower);
    if reads_content && has_rust_glob(&lower) {
        return true;
    }
    if inventory_command(&lower)
        && lower.contains(".rs")
        && (pipe_to_content_reader(&lower) || lower.contains("-exec "))
    {
        return true;
    }
    reads_content && lower.matches(".rs").count() > 1
}

fn rg_searches_rust_without_ingest(command: &str, lower: &str) -> bool {
    if !command_has_tool(lower, "rg") || is_rust_ingest_command(lower) {
        return false;
    }
    let rust_paths = command_candidate_paths(command)
        .into_iter()
        .filter(|path| rust_source_path_or_glob(path))
        .collect::<Vec<_>>();
    if !rust_paths.is_empty() && (rust_paths.len() > 1 || rg_has_context_dump(command)) {
        return true;
    }
    if rust_paths.len() == 1 && !has_rust_glob(lower) {
        return false;
    }
    if has_rust_glob(lower) || lower.contains("--type rust") || lower.contains("-t rust") {
        return true;
    }
    if rg_has_explicit_non_rust_target(command) {
        return false;
    }
    source_root_scope(lower, &["src", "tests", "benches", "examples"])
}

fn rg_has_context_dump(command: &str) -> bool {
    let tokens = shell_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        matches!(
            token.as_str(),
            "-A" | "-B"
                | "-C"
                | "--after-context"
                | "--before-context"
                | "--context"
                | "--passthru"
        ) || token.starts_with("-A")
            || token.starts_with("-B")
            || token.starts_with("-C")
            || token.starts_with("--after-context=")
            || token.starts_with("--before-context=")
            || token.starts_with("--context=")
            || matches!(
                tokens.get(index.saturating_sub(1)).map(String::as_str),
                Some("-A" | "-B" | "-C" | "--after-context" | "--before-context" | "--context")
            )
    })
}

pub(super) fn rg_has_explicit_non_rust_target(command: &str) -> bool {
    let tokens = shell_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        let glob_value = matches!(token.as_str(), "-g" | "--glob")
            .then(|| tokens.get(index + 1))
            .flatten();
        glob_value.is_some_and(|value| non_rust_path_or_glob(value))
            || token
                .strip_prefix("--glob=")
                .is_some_and(non_rust_path_or_glob)
            || non_rust_path_or_glob(token)
    })
}

fn non_rust_path_or_glob(token: &str) -> bool {
    let lower = token
        .trim_matches(|character| matches!(character, '\'' | '"' | ',' | ';'))
        .replace('\\', "/")
        .to_ascii_lowercase();
    if lower.starts_with('!')
        && (lower.contains(".rs") || lower.contains("*.rs") || lower.contains("**/*.rs"))
    {
        return true;
    }
    if lower.contains(".rs") || lower.contains("*.rs") || lower.contains("**/*.rs") {
        return false;
    }
    if !(lower.contains('/') || lower.contains("*.")) {
        return false;
    }
    lower.rsplit_once('.').is_some_and(|(_, extension)| {
        extension
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    })
}

pub(super) fn shell_rust_content_read_path(command: &str) -> Option<String> {
    let lower = command.replace('\\', "/").to_ascii_lowercase();
    let reads_shell_arg = command_has_content_reader(&lower);
    let reads_script_arg = script_runtime_reads_content(&lower);
    if !(reads_shell_arg || reads_script_arg) {
        return None;
    }
    if reads_script_arg && let Some(path) = embedded_rust_source_path(command) {
        return Some(path);
    }
    command_candidate_paths(command)
        .into_iter()
        .find(|path| rust_source_path_or_glob(path))
}

fn embedded_rust_source_path(command: &str) -> Option<String> {
    for part in command.split(|character: char| {
        character.is_whitespace()
            || matches!(
                character,
                '\'' | '"' | '`' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';'
            )
    }) {
        if rust_source_path_or_glob(part) {
            return Some(part.to_string());
        }
    }
    None
}

fn script_runtime_reads_content(command: &str) -> bool {
    if !command_has_script_runtime(command) {
        return false;
    }
    [
        "open(",
        "open ",
        "file.read",
        "file.open",
        "read_text(",
        ".read(",
        ".readline(",
        "readfile",
        "readfilesync",
        "fs.read",
    ]
    .into_iter()
    .any(|needle| command.contains(needle))
}

fn command_has_script_runtime(command: &str) -> bool {
    [
        "python", "python3", "ruby", "perl", "node", "nodejs", "deno", "bun",
    ]
    .into_iter()
    .any(|tool| command_has_tool(command, tool))
}

pub(super) fn has_rust_glob(command: &str) -> bool {
    shell_tokens(command).iter().any(|token| {
        let token = token.strip_prefix("--glob=").unwrap_or(token);
        !token.starts_with('!') && (token.contains("*.rs") || token.contains("**/*.rs"))
    })
}

fn inventory_command(command: &str) -> bool {
    (command_has_tool(command, "rg") && command.contains("--files"))
        || command_has_tool(command, "fd")
        || command_has_tool(command, "find")
        || (command_has_tool(command, "git") && command.contains("ls-files"))
}

fn pipe_to_content_reader(command: &str) -> bool {
    command.split('|').skip(1).any(command_has_content_reader)
}

fn command_has_content_reader(command: &str) -> bool {
    rtk_reads_content(command)
        || [
            "cat", "bat", "less", "more", "head", "tail", "nl", "awk", "sed",
        ]
        .into_iter()
        .any(|tool| command_has_tool(command, tool))
}

fn rtk_reads_content(command: &str) -> bool {
    let tokens = shell_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        token == "rtk" && rtk_subcommand(&tokens[index + 1..]) == Some("read")
    })
}

fn rtk_subcommand(tokens: &[String]) -> Option<&str> {
    tokens
        .iter()
        .map(String::as_str)
        .find(|token| !token.starts_with('-'))
}
