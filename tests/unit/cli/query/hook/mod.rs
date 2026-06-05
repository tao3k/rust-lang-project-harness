use std::fs;
use std::path::Path;

use tempfile::TempDir;

use crate::cli::support::{normalize_temp_root, run_cli, write_search_fixture};
mod code;
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
            .map_or(true, Vec::is_empty),
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
