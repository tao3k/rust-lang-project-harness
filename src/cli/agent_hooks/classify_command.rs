use super::model::{rust_config_files, ts_config_files, ts_extensions};

pub(super) fn command_candidate_paths(command: &str) -> Vec<String> {
    command
        .split_whitespace()
        .map(|word| word.trim_matches(|character| matches!(character, '\'' | '"' | ',')))
        .filter(|word| !word.starts_with('-'))
        .filter(|word| {
            word.contains('/')
                || word.contains('\\')
                || word.ends_with(".rs")
                || ts_extensions().iter().any(|ext| word.ends_with(ext))
                || known_config_file(word)
        })
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn source_root_scope(command: &str, roots: &[&str]) -> bool {
    roots.iter().any(|root| {
        command.contains(&format!(" {root} "))
            || command.ends_with(&format!(" {root}"))
            || command.contains(&format!(" {root}/"))
    })
}

pub(super) fn command_has_tool(command: &str, tool: &str) -> bool {
    command
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '-')
        .any(|part| part == tool)
}

pub(super) fn is_rust_ingest_command(command: &str) -> bool {
    command
        .to_ascii_lowercase()
        .contains("rs-harness search ingest")
}

pub(super) fn path_has_known_extension(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    path.ends_with(".rs")
        || path.ends_with(".d.ts")
        || ts_extensions().iter().any(|ext| path.ends_with(ext))
}

pub(super) fn known_config_file(path: &str) -> bool {
    let path = path.replace('\\', "/").to_ascii_lowercase();
    rust_config_files()
        .iter()
        .chain(ts_config_files().iter())
        .any(|file| path.ends_with(file))
}

pub(super) fn rust_source_path_or_glob(text: &str) -> bool {
    let lower = text
        .trim_matches(|character| matches!(character, '\'' | '"' | ',' | ';'))
        .replace('\\', "/")
        .to_ascii_lowercase();
    if lower.starts_with('!') || lower.contains("/!") {
        return false;
    }
    lower.ends_with(".rs") || lower.contains("*.rs") || lower.contains("**/*.rs")
}

pub(super) fn shell_tokens(command: &str) -> Vec<String> {
    command
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|character| matches!(character, '\'' | '"' | ',' | ';'))
                .to_string()
        })
        .filter(|word| !word.is_empty())
        .collect()
}
