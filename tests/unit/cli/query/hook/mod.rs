use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

use crate::cli::support::{normalize_temp_root, run_cli, write_search_fixture};
mod code;
mod exact;
mod range;
mod selector;

#[test]
fn cli_query_hook_line_range_without_code_outputs_read_plan() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"hook-line-range-frontier\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write manifest");
    fs::write(
        root.join("src/lib.rs"),
        r#"fn first() {
    let decision = classify_hook();
}

fn second() {
    let output = run_facade();
}
"#,
    )
    .expect("write source");

    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:1:7".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.starts_with("[read-plan] "), "{stdout}");
    assert!(stdout.contains("selector=src/lib.rs:1:7"), "{stdout}");
    assert!(stdout.contains("alg=symbol-frontier"), "{stdout}");
    assert!(stdout.contains("syn=function_item/name"), "{stdout}");
    assert!(stdout.contains("frontier="), "{stdout}");
    assert!(stdout.contains("omit=code"), "{stdout}");
    assert!(
        stdout.contains("avoid=repeat-wide-read,manual-window-scan,raw-read"),
        "{stdout}"
    );
    assert!(!stdout.contains("[search-owner]"), "{stdout}");
    assert!(!stdout.contains("|item "), "{stdout}");
    assert!(!stdout.contains("fn first()"), "{stdout}");
    assert!(
        !stdout.contains("let decision = classify_hook();"),
        "{stdout}"
    );
}

#[test]
fn cli_query_hook_line_range_code_outputs_source_slice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"hook-line-range\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write manifest");
    fs::write(
        root.join("src/lib.rs"),
        r#"fn first() {
    assert_eq!(
        decision.routes[0].argv,
        [
            "py-harness",
            "query",
            "--selector",
            "src/tools/report.py",
            ".",
        ],
    );
}

fn second() {
    let decision = classify_hook();
}
"#,
    )
    .expect("write source");

    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:7:14".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    insta::assert_snapshot!(
        stdout.trim_end(),
        @r###"
            "--selector",
            "src/tools/report.py",
            ".",
        ],
    );
}

fn second() {
"###
    );
    assert!(!stdout.contains("|fact"), "{stdout}");
}

#[test]
fn cli_query_hook_source_option_reads_worktree_index_and_head() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"hook-source-version\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write manifest");
    fs::write(
        root.join("src/lib.rs"),
        "pub fn marker() -> &'static str {\n    \"head\"\n}\n",
    )
    .expect("write head source");
    run_git(root, &["init"]);
    run_git(root, &["config", "user.email", "ci@example.invalid"]);
    run_git(root, &["config", "user.name", "CI"]);
    run_git(root, &["add", "Cargo.toml", "src/lib.rs"]);
    run_git(root, &["commit", "-m", "initial"]);
    fs::write(
        root.join("src/lib.rs"),
        "pub fn marker() -> &'static str {\n    \"index\"\n}\n",
    )
    .expect("write index source");
    run_git(root, &["add", "src/lib.rs"]);
    fs::write(
        root.join("src/lib.rs"),
        "pub fn marker() -> &'static str {\n    \"worktree\"\n}\n",
    )
    .expect("write worktree source");

    let worktree_stdout = query_source_stdout(root, &[]);
    assert!(
        worktree_stdout.contains("\"worktree\""),
        "{worktree_stdout}"
    );
    assert!(!worktree_stdout.contains("\"index\""), "{worktree_stdout}");
    assert!(!worktree_stdout.contains("\"head\""), "{worktree_stdout}");

    let index_stdout = query_source_stdout(root, &["--source", "index"]);
    assert!(index_stdout.contains("\"index\""), "{index_stdout}");
    assert!(!index_stdout.contains("\"worktree\""), "{index_stdout}");
    assert!(!index_stdout.contains("\"head\""), "{index_stdout}");

    let head_stdout = query_source_stdout(root, &["--source", "head"]);
    assert!(head_stdout.contains("\"head\""), "{head_stdout}");
    assert!(!head_stdout.contains("\"worktree\""), "{head_stdout}");
    assert!(!head_stdout.contains("\"index\""), "{head_stdout}");

    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:1:3".as_ref(),
        "--source".as_ref(),
        "index".as_ref(),
        "--code".as_ref(),
        "--view".as_ref(),
        "read-packet".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let value = serde_json::from_slice::<serde_json::Value>(&output.stdout).expect("read json");
    assert_eq!(value["sourceVersion"], "index");
    let canonical_root = fs::canonicalize(root).expect("canonical root");
    assert_eq!(
        value["repositoryRoot"].as_str().expect("repository root"),
        canonical_root.display().to_string()
    );
    assert!(
        value["gitBlobOid"]
            .as_str()
            .is_some_and(|oid| oid.len() >= 40),
        "{value}"
    );
    assert_eq!(
        value["sourceWindows"][0]["text"].as_str(),
        Some(index_stdout.trim_end())
    );
}

#[test]
fn cli_query_hook_selector_follows_workspace_path_dependency_roots() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/hook\"]\n\n[workspace.dependencies]\nrust-lang-project-harness = { path = \"languages/rust-lang-project-harness\", default-features = false }\n",
    )
    .expect("write root manifest");
    fs::create_dir_all(root.join("crates/hook")).expect("create hook crate");
    fs::write(
        root.join("crates/hook/Cargo.toml"),
        "[package]\nname = \"hook\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies]\nrust-lang-project-harness = { workspace = true }\n",
    )
    .expect("write hook manifest");
    fs::create_dir_all(root.join("languages/rust-lang-project-harness"))
        .expect("create harness crate");
    fs::write(
        root.join("languages/rust-lang-project-harness/Cargo.toml"),
        "[package]\nname = \"rust-lang-project-harness\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write harness manifest");
    fs::create_dir_all(root.join("crates/hook/src")).expect("create hook src");
    fs::write(root.join("crates/hook/src/lib.rs"), "pub fn hook() {}\n")
        .expect("write hook source");
    fs::create_dir_all(root.join("languages/rust-lang-project-harness/src"))
        .expect("create harness src");
    fs::write(
        root.join("languages/rust-lang-project-harness/src/lib.rs"),
        "pub fn harness() {}\n",
    )
    .expect("write harness source");
    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "languages/rust-lang-project-harness/src/lib.rs".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(
        stdout.starts_with(
            "[search-owner] q=languages/rust-lang-project-harness/src/lib.rs pkg=languages/rust-lang-project-harness own=1 item=1"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("|item harness kind=fn"), "{stdout}");
}

fn query_source_stdout(root: &Path, source_args: &[&str]) -> String {
    let mut args = vec![
        "query",
        "--from-hook",
        "direct-source-read",
        "--selector",
        "src/lib.rs:1:3",
    ];
    args.extend(source_args);
    args.push("--code");
    let output = run_cli(
        args.into_iter()
            .map(std::ffi::OsString::from)
            .chain([root.as_os_str().to_os_string()]),
    );
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

fn run_git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("run git");
    assert!(output.status.success(), "{output:?}");
}

#[test]
fn cli_query_code_output_strips_workspace_prefixed_selector_for_package_root() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "languages/rust-lang-project-harness/tests/unit/cli/support.rs".as_ref(),
        "--query".as_ref(),
        "run_search".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("fn run_search"), "{stdout}");
    assert!(stdout.contains("command_args.push"), "{stdout}");
    assert!(stdout.contains("command_args.extend"), "{stdout}");
    assert!(stdout.contains("normalize_temp_root"), "{stdout}");

    let relative_root_output = run_cli([
        "query",
        "--from-hook",
        "direct-source-read",
        "--selector",
        "languages/rust-lang-project-harness/tests/unit/cli/support.rs",
        "--query",
        "run_search",
        "--code",
        ".",
    ]);
    assert!(
        relative_root_output.status.success(),
        "{relative_root_output:?}"
    );
    let relative_stdout = String::from_utf8(relative_root_output.stdout).expect("utf8 stdout");
    assert!(
        relative_stdout.contains("fn run_search"),
        "{relative_stdout}"
    );
    assert!(
        relative_stdout.contains("command_args.push"),
        "{relative_stdout}"
    );
    assert!(
        relative_stdout.contains("command_args.extend"),
        "{relative_stdout}"
    );

    let temp = TempDir::new().expect("temp dir");
    let package_root = temp.path().join("rust-lang-project-harness");
    fs::create_dir_all(package_root.join("tests/unit/cli")).expect("create test fixture dir");
    fs::write(
        package_root.join("Cargo.toml"),
        "[package]\nname = \"rust-lang-project-harness\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write manifest");
    fs::write(
        package_root.join("tests/unit/cli/support.rs"),
        r#"fn normalize_temp_root(rendered: &str) -> String {
    rendered.to_string()
}

fn run_search() -> String {
    let mut command_args = Vec::new();
    command_args.push("search");
    command_args.extend(["owner"]);
    normalize_temp_root("ok")
}
"#,
    )
    .expect("write support fixture");

    let standalone_output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "languages/rust-lang-project-harness/tests/unit/cli/support.rs".as_ref(),
        "--query".as_ref(),
        "run_search".as_ref(),
        "--code".as_ref(),
        package_root.as_os_str(),
    ]);
    assert!(standalone_output.status.success(), "{standalone_output:?}");
    let standalone_stdout = String::from_utf8(standalone_output.stdout).expect("utf8 stdout");
    assert!(
        standalone_stdout.contains("fn run_search"),
        "{standalone_stdout}"
    );
    assert!(
        standalone_stdout.contains("command_args.push"),
        "{standalone_stdout}"
    );
    assert!(
        standalone_stdout.contains("command_args.extend"),
        "{standalone_stdout}"
    );
}

#[test]
fn cli_query_hook_selector_json_can_emit_provider_read_packet() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "owner:src/lib.rs:4:8".as_ref(),
        "--view".as_ref(),
        "read-packet".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let value = serde_json::from_slice::<serde_json::Value>(&output.stdout).expect("read json");
    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-read-packet"
    );
    assert_eq!(
        value["protocolId"],
        "agent.semantic-protocols.semantic-language"
    );
    assert_eq!(value["method"], "query/direct-source-read");
    assert_eq!(value["selector"], "src/lib.rs:4:8");
    assert_eq!(value["outputMode"], "read-packet");
    assert!(
        value
            .get("sourceWindows")
            .and_then(|windows| windows.as_array())
            .is_none_or(Vec::is_empty),
        "{value}"
    );
    let read_plan = value["readPlan"].as_object().expect("read plan");
    assert_eq!(read_plan["mode"], "range-frontier");
    assert_eq!(read_plan["code"], false);
    assert_eq!(read_plan["reason"], "locator-frontier");
    assert_eq!(read_plan["algorithm"], "symbol-frontier");
    assert_eq!(read_plan["syn"], "function_item/name");
    assert_eq!(read_plan["symbols"][0]["itemName"], "load");
    assert_eq!(read_plan["symbols"][0]["itemKind"], "fn");
    assert_eq!(read_plan["symbols"][0]["read"], "src/lib.rs:6:6");
    assert_eq!(read_plan["frontier"][0]["kind"], "symbol");
    assert_eq!(read_plan["frontier"][0]["read"], "src/lib.rs:6:6");
    assert_eq!(
        value["syntaxQueryRef"],
        "semantic-tree-sitter-query/rust-owner-items.v1"
    );
    assert_eq!(value["syntaxAnchor"]["nodeType"], "function_item");
    assert_eq!(value["syntaxAnchor"]["capture"], "function.name");
}
