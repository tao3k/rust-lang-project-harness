use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use crate::cli::support::{run_cli, write_search_fixture};

#[test]
fn cli_query_hook_wide_line_range_code_outputs_source_slice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);
    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:1:80".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(!stdout.starts_with("[read-plan] "), "{stdout}");
    assert!(!stdout.contains("code=false"), "{stdout}");
    assert!(!stdout.contains("|range "), "{stdout}");
    assert!(!stdout.contains("|symbol "), "{stdout}");
    assert!(!stdout.contains("|window "), "{stdout}");
    assert!(!stdout.contains("[search-owner]"), "{stdout}");
    assert!(stdout.starts_with("//! Test crate.\n"), "{stdout}");
    assert!(stdout.contains("mod domain;\n"), "{stdout}");
    assert!(
        stdout.contains("pub fn load() -> Thing { domain::make_thing() }\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "impl WireApi for PublicWire { fn wire(&self) -> anyhow::Result<Thing> { todo!() } }\n"
        ),
        "{stdout}"
    );
}

#[test]
fn cli_query_hook_workspace_line_range_code_outputs_workspace_source_slice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let member_root = root.join("crates/agent-semantic-client-core");
    fs::create_dir_all(member_root.join("src")).expect("create member src");
    fs::write(
        member_root.join("Cargo.toml"),
        "[package]\nname = \"agent-semantic-client-core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write member manifest");
    fs::write(
        member_root.join("src/receipt.rs"),
        "pub struct ProviderCommandReceipt {\n    pub argv: Vec<String>,\n}\n",
    )
    .expect("write workspace source");

    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--workspace".as_ref(),
        "--selector".as_ref(),
        "crates/agent-semantic-client-core/src/receipt.rs:1:3".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        stdout.starts_with("pub struct ProviderCommandReceipt {\n"),
        "{stdout}"
    );
    assert!(stdout.contains("pub argv: Vec<String>,\n"), "{stdout}");
}

#[test]
fn cli_query_hook_wide_line_range_json_returns_source_window_packet() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:1:80".as_ref(),
        "--code".as_ref(),
        "--view".as_ref(),
        "read-packet".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let value = serde_json::from_slice::<Value>(&output.stdout).expect("read json");
    assert_eq!(value["schemaVersion"], "1");
    assert_eq!(value["selector"], "src/lib.rs:1:80");
    assert!(value.get("readPlan").is_none(), "{value}");

    let windows = value["sourceWindows"].as_array().expect("source windows");
    assert_eq!(windows.len(), 1, "{value}");
    let window = &windows[0];
    assert_eq!(window["read"], "src/lib.rs:1:80");
    assert_eq!(window["lines"][0]["number"], 1);
    assert_eq!(window["lines"][0]["text"], "//! Test crate.");
    assert!(
        window["text"]
            .as_str()
            .expect("window text")
            .contains("pub fn load() -> Thing { domain::make_thing() }\n"),
        "{value}"
    );
}
