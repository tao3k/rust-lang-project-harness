use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::classify_command::{
    command_candidate_paths, command_has_tool, is_rust_ingest_command, known_config_file,
    path_has_known_extension, rust_source_path_or_glob, shell_tokens, source_root_scope,
};
use super::classify_shell::{
    has_rust_glob, rg_has_explicit_non_rust_target, shell_bulk_reads_rust,
    shell_rust_content_read_path,
};
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
    if is_shell_tool(payload) {
        if shell_bulk_reads_rust(command) {
            return Some(rust_bulk_pipe_flow());
        }
        return shell_rust_content_read_path(command).map(|path| rust_direct_read_flow(&path));
    }
    None
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
    if command_has_tool(command, "rg")
        && rg_has_explicit_non_rust_target(command)
        && !has_rust_glob(&command.to_ascii_lowercase())
    {
        profiles.remove(&Profile::Rust);
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
    let mut commands = Vec::<String>::new();
    collect_commands_from_tool_input(&payload.tool_input, &mut commands);
    commands.join("\n")
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
        if raw_search_path_targets_profile(&path) {
            profiles.extend(path_profiles(&path, project));
        }
    }
    if command_targets_workspace_root(command)
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

fn command_targets_workspace_root(command: &str) -> bool {
    shell_tokens(command)
        .iter()
        .any(|token| matches!(token.as_str(), "." | "./"))
}

fn raw_search_path_targets_profile(path: &str) -> bool {
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    if lower.starts_with('!') || lower.contains("/!") {
        return false;
    }
    lower.ends_with(".rs")
        || lower.contains("*.rs")
        || lower.contains("**/*.rs")
        || lower.ends_with(".d.ts")
        || ts_extensions().iter().any(|ext| lower.ends_with(ext))
        || ts_extensions()
            .iter()
            .any(|ext| lower.contains(&format!("*{ext}")))
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

fn collect_paths_from_tool_input(value: &Value, files: &mut BTreeSet<String>) {
    files.extend(collect_path_candidates(value));
}

fn collect_path_candidates(value: &Value) -> Vec<String> {
    match value {
        Value::String(text) if path_has_known_extension(text) || known_config_file(text) => {
            vec![text.to_string()]
        }
        Value::Array(values) => values.iter().flat_map(collect_path_candidates).collect(),
        Value::Object(fields) => fields
            .iter()
            .flat_map(|(key, value)| collect_keyed_path_candidates(key, value))
            .collect(),
        _ => Vec::new(),
    }
}

fn collect_keyed_path_candidates(key: &str, value: &Value) -> Vec<String> {
    if path_key(key) {
        return collect_path_value_candidates(value);
    }
    if path_collection_key(key) {
        return collect_path_candidates(value);
    }
    Vec::new()
}

fn collect_path_value_candidates(value: &Value) -> Vec<String> {
    match value {
        Value::String(path) if path_has_known_extension(path) || known_config_file(path) => {
            vec![path.to_string()]
        }
        Value::Array(values) => values
            .iter()
            .flat_map(collect_path_value_candidates)
            .collect(),
        _ => Vec::new(),
    }
}

pub(super) fn is_shell_tool(payload: &HookPayload) -> bool {
    let Some(tool) = payload.tool_name.as_deref() else {
        return payload.tool_input.get("command").is_some()
            || payload.tool_input.get("cmd").is_some();
    };
    shell_tool_name(tool) || nested_shell_tool(&payload.tool_input)
}

fn is_edit_tool(payload: &HookPayload) -> bool {
    payload.tool_name.as_deref().is_some_and(|tool| {
        let lower = tool.to_ascii_lowercase();
        matches!(
            tool_leaf(tool),
            "apply_patch" | "Edit" | "Write" | "FsWriteFile" | "FsRemove" | "FsCopy"
        ) || matches!(
            lower.as_str(),
            "writefile"
                | "fs.write"
                | "fs/write"
                | "fs.writefile"
                | "fs/writefile"
                | "fs.remove"
                | "fs/remove"
                | "fs.copy"
                | "fs/copy"
                | "fs.rename"
                | "fs/rename"
        )
    })
}

fn command_has_raw_search_tool(command: &str) -> bool {
    ["rg", "fd", "grep", "find", "ast-grep"]
        .into_iter()
        .any(|tool| command_has_tool(command, tool))
}

fn is_ts_ingest_command(command: &str) -> bool {
    command
        .to_ascii_lowercase()
        .contains("ts-harness search ingest")
}

fn is_read_tool(payload: &HookPayload) -> bool {
    payload.tool_name.as_deref().is_some_and(|tool| {
        let lower = tool.to_ascii_lowercase();
        let leaf = tool_leaf(tool);
        matches!(
            leaf,
            "Read" | "read" | "read_file" | "readFile" | "FsRead" | "FsReadFile"
        ) || matches!(
            lower.as_str(),
            "fs.read" | "fs.readbin" | "fs.readfile" | "fs/readfile"
        ) || lower.ends_with("__read_file")
            || (lower.starts_with("mcp__") && lower.contains("__read"))
    })
}

fn rust_source_path_from_value(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => rust_source_path_or_glob(text).then(|| text.to_string()),
        Value::Array(values) => values.iter().find_map(rust_source_path_from_value),
        Value::Object(fields) => fields.iter().find_map(|(key, value)| {
            if path_key(key) {
                return rust_source_path_from_path_value(value);
            }
            if path_collection_key(key) {
                return rust_source_path_from_value(value);
            }
            None
        }),
        _ => None,
    }
}

fn rust_source_path_from_path_value(value: &Value) -> Option<String> {
    match value {
        Value::String(path) => rust_source_path_or_glob(path).then(|| path.to_string()),
        Value::Array(values) => values.iter().find_map(rust_source_path_from_path_value),
        _ => None,
    }
}

fn path_key(key: &str) -> bool {
    matches!(
        normalized_key(key).as_str(),
        "path" | "file" | "filename" | "filepath" | "absolutepath" | "relativepath" | "uri"
    )
}

fn path_collection_key(key: &str) -> bool {
    matches!(
        normalized_key(key).as_str(),
        "paths" | "files" | "filenames" | "filepaths" | "uris"
    )
}

fn normalized_key(key: &str) -> String {
    key.chars()
        .filter(|character| !matches!(character, '_' | '-' | '.'))
        .flat_map(char::to_lowercase)
        .collect()
}

fn command_key(key: &str) -> bool {
    matches!(normalized_key(key).as_str(), "command" | "cmd")
}

fn collect_commands_from_tool_input(value: &Value, commands: &mut Vec<String>) {
    match value {
        Value::Array(values) => {
            for value in values {
                collect_commands_from_tool_input(value, commands);
            }
        }
        Value::Object(fields) => {
            for (key, value) in fields {
                if command_key(key)
                    && let Some(command) = value.as_str()
                {
                    commands.push(command.to_string());
                    continue;
                }
                collect_commands_from_tool_input(value, commands);
            }
        }
        _ => {}
    }
}

fn nested_shell_tool(value: &Value) -> bool {
    match value {
        Value::Array(values) => values.iter().any(nested_shell_tool),
        Value::Object(fields) => {
            let current = ["recipient_name", "tool_name", "name"]
                .into_iter()
                .filter_map(|key| fields.get(key))
                .filter_map(Value::as_str)
                .any(shell_tool_name);
            current || fields.values().any(nested_shell_tool)
        }
        _ => false,
    }
}

fn shell_tool_name(tool: &str) -> bool {
    let leaf = tool_leaf(tool);
    matches!(
        leaf,
        "Bash" | "bash" | "exec" | "exec_command" | "command_execution" | "shell" | "run_command"
    )
}

fn tool_leaf(tool: &str) -> &str {
    tool.rsplit(['.', '/']).next().unwrap_or(tool)
}

fn rust_direct_read_flow(path: &str) -> String {
    format!(
        "[rs-harness-flow] blocked=read-rs path={path} policy=search-first route=owner\n\
         |owner run=`rs-harness search owner {path} items --trace --view seeds --seeds 8 .` tests=`rs-harness search tests {path} --view seeds --seeds 4 .`\n\
         |rule no raw Rust source reads; use agentHookDecision.routes; one-search-command-at-a-time"
    )
}

fn rust_bulk_pipe_flow() -> String {
    "[rs-harness-flow] blocked=bulk-rs-dump reason=\"Raw broad Rust search\" policy=pipe-to-ingest\n\
     |flow guide=prime->rg-or-paths->ingest->owner-or-deps token=bounded run=`rg -n \"<query>\" --glob '*.rs' src tests | rs-harness search ingest items tests --view seeds --seeds 8 .`\n\
     |next owner=`rs-harness search owner <seed-owner> items --trace --view seeds --seeds 8 .` deps=`rs-harness search deps <dep[/path][::api]> public-api --trace --view seeds --seeds 6 .` rule=no-raw-rust-dumps,one-search-command-at-a-time subagent=[search-subagent]"
        .to_string()
}

pub(super) fn rust_command_guide(owner_path: &str, project_root: &str) -> String {
    format!(
        "|flow guide=prime->batch-or-owner->tests->edit token=bounded\n\
         |prime run=`rs-harness search prime --view seeds --seeds 8 {project_root}`\n\
         |batch run=`printf '%s\\n' <paths...> | rs-harness search ingest items tests --view seeds --seeds 8 {project_root}`\n\
         |owner run=`rs-harness search owner {owner_path} items --trace --view seeds --seeds 8 {project_root}`\n\
         |tests run=`rs-harness search tests {owner_path} --view seeds --seeds 4 {project_root}`\n\
         |rule one-search-command-at-a-time; installed-binary-only; no `&&`; follow `|seed` and `next=`; do not Read Rust source files directly\n\
         |subagent assign one bounded packet; require `[search-subagent] role=... evidence=... missing=... next=... risk=...`"
    )
}
