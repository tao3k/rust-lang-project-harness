use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::model::{
    HookPayload, Profile, RUST_CHECK, TS_CHECK, rust_config_files, rust_roots, ts_config_files,
    ts_extensions, ts_roots,
};
use super::policy::CodexHookPolicy;
use super::project::ProjectProfiles;

pub(super) fn bulk_rust_read_reason(
    payload: &HookPayload,
    command: &str,
    policy: &CodexHookPolicy,
    project: &ProjectProfiles,
) -> Option<String> {
    if !project.rust.enabled || !policy.profile(Profile::Rust).raw_search_requires_ingest {
        return None;
    }
    if is_read_tool(payload)
        && let Some(path) = rust_source_path_from_value(&payload.tool_input)
    {
        return Some(rust_direct_read_flow(&path));
    }
    shell_bulk_reads_rust(command).then(rust_bulk_pipe_flow)
}

pub(super) fn broad_raw_search_profiles(
    command: &str,
    policy: &CodexHookPolicy,
    project: &ProjectProfiles,
) -> BTreeSet<Profile> {
    if command.trim().is_empty() {
        return BTreeSet::new();
    }
    let mut profiles = raw_command_profiles(command, project);
    if !command_has_raw_search_tool(command) || exact_file_search(command) {
        return BTreeSet::new();
    }
    if is_rust_ingest_command(command) {
        profiles.remove(&Profile::Rust);
    }
    if is_ts_ingest_command(command) {
        profiles.remove(&Profile::TypeScript);
    }
    if !policy.global.raw_ast_grep_blocked && command_has_tool(command, "ast-grep") {
        return BTreeSet::new();
    }
    profiles
        .into_iter()
        .filter(|profile| policy.profile(*profile).raw_search_requires_ingest)
        .collect()
}

pub(super) fn touched_files_by_profile(
    payload: &HookPayload,
    command: &str,
    policy: &CodexHookPolicy,
    project: &ProjectProfiles,
) -> BTreeMap<Profile, Vec<String>> {
    let mut by_profile = BTreeMap::<Profile, Vec<String>>::new();
    if !is_edit_tool(payload) && !command.contains("*** ") {
        return by_profile;
    }
    for path in touched_paths(payload, command) {
        let profiles = path_profiles(&path, project);
        if profiles.is_empty() && policy.global.docs_only_exception {
            continue;
        }
        for profile in profiles {
            by_profile.entry(profile).or_default().push(path.clone());
        }
    }
    by_profile
}

pub(super) fn command_evidence_profiles(command: &str) -> BTreeSet<Profile> {
    let lower = command.to_ascii_lowercase();
    [
        (lower.contains("rs-harness search "), Profile::Rust),
        (lower.contains("ts-harness search "), Profile::TypeScript),
    ]
    .into_iter()
    .filter_map(|(matched, profile)| matched.then_some(profile))
    .collect()
}

pub(super) fn changed_check_profiles(command: &str) -> BTreeSet<Profile> {
    let lower = command.to_ascii_lowercase();
    [
        (lower.contains(RUST_CHECK), Profile::Rust),
        (lower.contains(TS_CHECK), Profile::TypeScript),
    ]
    .into_iter()
    .filter_map(|(matched, profile)| matched.then_some(profile))
    .collect()
}

pub(super) fn raw_search_reason(profiles: &BTreeSet<Profile>) -> &'static str {
    match (
        profiles.contains(&Profile::Rust),
        profiles.contains(&Profile::TypeScript),
    ) {
        (true, true) => {
            "Broad search crosses harness profiles. Use profile-specific ingest through rs-harness or ts-harness."
        }
        (true, false) => {
            "Raw broad Rust search must enter the rs-harness flow: `rg -n \"<query>\" --glob '*.rs' src tests | rs-harness search ingest items tests .`; if scope is unclear run `rs-harness search text <query> --explain --view seeds --seeds 6 .` and follow `|seed`/`next=`."
        }
        (false, true) => {
            "Raw broad TS/JS search must pipe to harness ingest: `rg -n \"<query>\" --glob '*.{ts,tsx,js,jsx,mts,cts,mjs,cjs}' src tests apps packages | ts-harness search ingest`."
        }
        (false, false) => "No relevant harness profile matched.",
    }
}

pub(super) fn prime_required_reason(profiles: &BTreeSet<Profile>) -> &'static str {
    match (
        profiles.contains(&Profile::Rust),
        profiles.contains(&Profile::TypeScript),
    ) {
        (true, true) => {
            "Run Rust and TS/JS prime or focused owner search before editing mixed profile files."
        }
        (true, false) => {
            "Run search flow before editing Rust: `rs-harness search prime --view seeds --seeds 8 .`, then `rs-harness search owner <path-or-owner> items --explain --view seeds .` or `rg -n \"<query>\" --glob '*.rs' src tests | rs-harness search ingest items tests .`."
        }
        (false, true) => {
            "Run `ts-harness search prime` or `ts-harness search owner <path>` before editing TS/JS files."
        }
        (false, false) => "No relevant harness profile matched.",
    }
}

pub(super) fn changed_check_reason(profiles: &[Profile]) -> String {
    if profiles.len() == 1 {
        return format!(
            "{} files changed. Run `{}`.",
            profiles[0].display(),
            profiles[0].check_command()
        );
    }
    let commands = profiles
        .iter()
        .map(|profile| profile.check_command())
        .collect::<Vec<_>>()
        .join("\\n");
    format!("Rust and TS/JS files changed. Run:\\n{commands}")
}

pub(super) fn touched_file_count(touched: &BTreeMap<Profile, Vec<String>>) -> usize {
    touched
        .values()
        .flat_map(|files| files.iter())
        .collect::<BTreeSet<_>>()
        .len()
}

pub(super) fn tool_command(payload: &HookPayload) -> String {
    payload
        .tool_input
        .get("command")
        .or_else(|| payload.tool_input.get("cmd"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn raw_command_profiles(command: &str, project: &ProjectProfiles) -> BTreeSet<Profile> {
    let lower = command.to_ascii_lowercase().replace('\\', "/");
    let mut profiles = BTreeSet::new();
    if (lower.contains("*.rs") || lower.contains(" -e rs") || lower.contains("src/"))
        && project.rust.enabled
    {
        profiles.insert(Profile::Rust);
    }
    if (ts_extensions().iter().any(|ext| lower.contains(ext))
        || lower.contains("*.{ts,tsx")
        || lower.contains("packages")
        || lower.contains("apps"))
        && project.typescript.enabled
    {
        profiles.insert(Profile::TypeScript);
    }
    for path in command_candidate_paths(command) {
        profiles.extend(path_profiles(&path, project));
    }
    if lower.contains(" .")
        || lower.ends_with(" .")
        || source_root_scope(&lower, rust_roots())
        || source_root_scope(&lower, ts_roots())
    {
        profiles.extend(project.enabled_profiles());
    }
    if command_has_tool(command, "ast-grep") && profiles.is_empty() {
        profiles.extend(project.enabled_profiles());
    }
    profiles
}

fn exact_file_search(command: &str) -> bool {
    if !(command_has_tool(command, "rg")
        || command_has_tool(command, "grep")
        || command_has_tool(command, "ast-grep"))
    {
        return false;
    }
    let paths = command_candidate_paths(command);
    paths.len() == 1 && path_has_known_extension(&paths[0])
}

fn command_candidate_paths(command: &str) -> Vec<String> {
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

fn touched_paths(payload: &HookPayload, command: &str) -> BTreeSet<String> {
    let mut files = BTreeSet::<String>::new();
    collect_paths_from_tool_input(&payload.tool_input, &mut files);
    for line in command.lines() {
        for prefix in [
            "*** Update File: ",
            "*** Add File: ",
            "*** Delete File: ",
            "*** Move to: ",
        ] {
            if let Some(path) = line.strip_prefix(prefix) {
                files.insert(path.trim().to_string());
            }
        }
    }
    files
}

fn path_profiles(path: &str, project: &ProjectProfiles) -> BTreeSet<Profile> {
    let path = path.replace('\\', "/");
    let lower = path.to_ascii_lowercase();
    let mut profiles = BTreeSet::new();
    if excluded_path(&lower) {
        return profiles;
    }
    if project.rust.enabled && rust_path(&lower) {
        profiles.insert(Profile::Rust);
    }
    if project.typescript.enabled && ts_path(&lower) {
        profiles.insert(Profile::TypeScript);
    }
    profiles
}

fn rust_path(path: &str) -> bool {
    path.ends_with(".rs") || rust_config_files().iter().any(|file| path.ends_with(file))
}

fn ts_path(path: &str) -> bool {
    ts_extensions().iter().any(|ext| path.ends_with(ext))
        || ts_config_files().iter().any(|file| path.ends_with(file))
        || path.ends_with(".d.ts")
}

fn excluded_path(path: &str) -> bool {
    [
        "target/",
        "vendor/",
        "node_modules/",
        "dist/",
        "build/",
        "coverage/",
        ".next/",
        ".turbo/",
        "out/",
        ".cache/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix) || path.contains(&format!("/{prefix}")))
}

fn source_root_scope(command: &str, roots: &[&str]) -> bool {
    roots.iter().any(|root| {
        command.contains(&format!(" {root} "))
            || command.ends_with(&format!(" {root}"))
            || command.contains(&format!(" {root}/"))
    })
}

fn collect_paths_from_tool_input(value: &Value, files: &mut BTreeSet<String>) {
    match value {
        Value::String(text) if path_has_known_extension(text) || known_config_file(text) => {
            files.insert(text.to_string());
        }
        Value::Array(values) => {
            for value in values {
                collect_paths_from_tool_input(value, files);
            }
        }
        Value::Object(fields) => {
            for (key, value) in fields {
                if matches!(key.as_str(), "path" | "file" | "filename")
                    && let Some(path) = value.as_str()
                    && (path_has_known_extension(path) || known_config_file(path))
                {
                    files.insert(path.to_string());
                }
                collect_paths_from_tool_input(value, files);
            }
        }
        _ => {}
    }
}

fn is_edit_tool(payload: &HookPayload) -> bool {
    payload
        .tool_name
        .as_deref()
        .is_some_and(|tool| matches!(tool, "apply_patch" | "Edit" | "Write"))
}

fn command_has_raw_search_tool(command: &str) -> bool {
    ["fd", "grep", "find", "ast-grep"]
        .into_iter()
        .any(|tool| command_has_tool(command, tool))
}

fn command_has_tool(command: &str, tool: &str) -> bool {
    command
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '-')
        .any(|part| part == tool)
}

fn is_rust_ingest_command(command: &str) -> bool {
    command
        .to_ascii_lowercase()
        .contains("rs-harness search ingest")
}

fn is_ts_ingest_command(command: &str) -> bool {
    command
        .to_ascii_lowercase()
        .contains("ts-harness search ingest")
}

fn path_has_known_extension(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    path.ends_with(".rs")
        || path.ends_with(".d.ts")
        || ts_extensions().iter().any(|ext| path.ends_with(ext))
}

fn known_config_file(path: &str) -> bool {
    let path = path.replace('\\', "/").to_ascii_lowercase();
    rust_config_files()
        .iter()
        .chain(ts_config_files().iter())
        .any(|file| path.ends_with(file))
}

fn is_read_tool(payload: &HookPayload) -> bool {
    payload.tool_name.as_deref().is_some_and(|tool| {
        matches!(
            tool,
            "Read" | "read" | "read_file" | "mcp__filesystem__read_file"
        ) || tool.ends_with("__read_file")
            || (tool.starts_with("mcp__") && tool.contains("__read"))
    })
}

fn rust_source_path_from_value(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => rust_source_path_or_glob(text).then(|| text.to_string()),
        Value::Array(values) => values.iter().find_map(rust_source_path_from_value),
        Value::Object(fields) => fields.iter().find_map(|(key, value)| {
            let path_key = matches!(key.as_str(), "path" | "file" | "filename" | "file_path");
            if path_key
                && let Some(path) = value.as_str()
                && rust_source_path_or_glob(path)
            {
                return Some(path.to_string());
            }
            rust_source_path_from_value(value)
        }),
        _ => None,
    }
}

fn rust_source_path_or_glob(text: &str) -> bool {
    let lower = text.replace('\\', "/").to_ascii_lowercase();
    lower.ends_with(".rs") || lower.contains("*.rs") || lower.contains("**/*.rs")
}

fn shell_bulk_reads_rust(command: &str) -> bool {
    if command.trim().is_empty() {
        return false;
    }
    let lower = command.replace('\\', "/").to_ascii_lowercase();
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

fn has_rust_glob(command: &str) -> bool {
    command.contains("*.rs") || command.contains("**/*.rs")
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
    [
        "cat", "bat", "less", "more", "head", "tail", "nl", "awk", "sed",
    ]
    .into_iter()
    .any(|tool| command_has_tool(command, tool))
}

fn rust_direct_read_flow(path: &str) -> String {
    format!(
        "[rs-harness-flow] blocked=read-rs path={path} policy=search-first\n\
         |prefer run=`rs-harness search owner {path} items --trace --view both .`\n\
         |tests run=`rs-harness search tests {path} --view seeds --seeds 4 .`\n\
         |orient run=`rs-harness search prime --view seeds --seeds 8 .`\n\
         |plan run=`rs-harness search owner {path} --explain --view seeds --seeds 6 .`\n\
         |pipe run=`rg -n \"<query>\" --glob '*.rs' src tests | rs-harness search ingest items tests .`\n\
         |rule follow `|seed` and `next=` lines; do not Read Rust source files directly"
    )
}

fn rust_bulk_pipe_flow() -> String {
    "[rs-harness-flow] blocked=bulk-rs-dump policy=pipe-to-ingest\n\
     |prefer run=`rg -n \"<query>\" --glob '*.rs' src tests | rs-harness search ingest items tests .`\n\
     |paths run=`fd -e rs . src tests | rs-harness search ingest items tests .`\n\
     |after run=`rs-harness search owner <top-owner> items --trace --view both .`\n\
     |deps run=`rs-harness search deps <dep[/subpath][::api]> public-api --trace --view seeds --seeds 6 .`\n\
     |plan run=`rs-harness search text <query> --explain --view seeds --seeds 6 .`\n\
     |subagent assign only one bounded search packet and require `[search-subagent] role=... evidence=... missing=... next=... risk=...`"
        .to_string()
}
