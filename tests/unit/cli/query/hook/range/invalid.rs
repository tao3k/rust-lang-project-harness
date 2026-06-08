use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use crate::cli::support::run_cli;

#[test]
fn cli_query_hook_wide_line_range_without_parser_items_outputs_source_slice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"invalid-range\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write manifest");
    let mut source = String::from("pub fn broken(\n");
    for line in 2..=80 {
        source.push_str(&format!("// line {line}\n"));
    }
    fs::write(root.join("src/lib.rs"), source).expect("write source");

    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:1:80".as_ref(),
        "--code".as_ref(),
        "--workspace".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(!stdout.starts_with("[read-plan] "), "{stdout}");
    assert!(stdout.starts_with("pub fn broken(\n"), "{stdout}");
    assert!(stdout.contains("// line 40\n"), "{stdout}");
    assert!(stdout.contains("// line 80\n"), "{stdout}");
    assert!(!stdout.contains("|range "), "{stdout}");
    assert!(!stdout.contains("|symbol "), "{stdout}");
    assert!(!stdout.contains("|window "), "{stdout}");
}

#[test]
fn cli_query_hook_wide_line_range_json_without_parser_items_returns_source_window_packet() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"invalid-range-json\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write manifest");
    let mut source = String::from("pub fn broken(\n");
    for line in 2..=80 {
        source.push_str(&format!("// line {line}\n"));
    }
    fs::write(root.join("src/lib.rs"), source).expect("write source");

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
        "--workspace".as_ref(),
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
    assert_eq!(window["lineCount"], 80);
    assert_eq!(window["lines"][0]["text"], "pub fn broken(");
    assert_eq!(window["lines"][79]["text"], "// line 80");
    assert!(
        window["text"]
            .as_str()
            .expect("window text")
            .contains("// line 40\n"),
        "{value}"
    );
}
