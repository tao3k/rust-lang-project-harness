use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use crate::cli::support::{run_cli, write_search_fixture};

#[test]
fn cli_query_hook_line_range_code_outputs_local_window() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"line-range-window\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write manifest");
    fs::write(
        root.join("src/lib.rs"),
        r#"fn run_facade() -> std::process::Output {
    unimplemented!()
}

mod language {
    use super::run_facade;

    #[test]
    fn rust_facade_invokes_provider_query() {
        let output = run_facade();
        assert!(output.status.success());
    }
}
"#,
    )
    .expect("write lib");

    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:5:11".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(!stdout.contains("[search-owner]"), "{stdout}");
    assert!(!stdout.contains("[read-plan]"), "{stdout}");
    assert!(stdout.starts_with("mod language {\n"), "{stdout}");
    assert!(
        stdout.contains("    use super::run_facade;\n\n    #[test]\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("        let output = run_facade();\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("        assert!(output.status.success());\n"),
        "{stdout}"
    );
    assert!(!stdout.lines().any(|line| line == "}"), "{stdout}");

    let json_output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:5:11".as_ref(),
        "--code".as_ref(),
        "--view".as_ref(),
        "read-packet".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(json_output.status.success(), "{json_output:?}");
    let value = serde_json::from_slice::<Value>(&json_output.stdout).expect("read packet");
    assert!(value.get("readPlan").is_none(), "{value}");
    let windows = value["sourceWindows"].as_array().expect("source windows");
    assert_eq!(windows.len(), 1, "{value}");
    let window = &windows[0];
    assert_eq!(window["read"], "src/lib.rs:5:11");
    assert_eq!(window["lineCount"], 7);
    assert_eq!(window["lines"][0]["number"], 5);
    assert_eq!(window["lines"][0]["text"], "mod language {");
    assert!(
        window["text"]
            .as_str()
            .expect("window text")
            .contains("    use super::run_facade;\n\n    #[test]\n"),
        "{value}"
    );
}

#[test]
fn cli_query_hook_read_packet_does_not_guess_syntax_refs_for_direct_source_slice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:6:6".as_ref(),
        "--code".as_ref(),
        "--view".as_ref(),
        "read-packet".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let value = serde_json::from_slice::<Value>(&output.stdout).expect("read json");

    assert!(value.get("syntaxQueryRef").is_none(), "{value}");
    assert!(value.get("syntaxAnchor").is_none(), "{value}");
    let window = &value["sourceWindows"][0];
    assert_eq!(window["read"], "src/lib.rs:6:6");
    assert_eq!(window["location"]["lineRange"], "6:6");
    assert!(window.get("fields").is_none(), "{value}");
}
