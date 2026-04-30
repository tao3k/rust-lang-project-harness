use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

#[test]
fn cli_renders_compact_text_by_default() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-compact");
    write_clean_source(root);

    let output = run_cli([root.as_os_str()]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.starts_with("[ok]"), "{stdout}");
    assert!(!stdout.trim_start().starts_with('{'), "{stdout}");
}

#[test]
fn cli_json_flag_renders_structured_report() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-json");
    write_clean_source(root);

    let output = run_cli(["--json".as_ref(), root.as_os_str()]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("json report");
    assert_eq!(value["modules"].as_array().expect("modules").len(), 1);
    assert_eq!(value["findings"].as_array().expect("findings").len(), 0);
}

#[test]
fn cli_keeps_agent_advice_non_blocking() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-advice");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod owned;\n").expect("write lib");
    fs::write(
        root.join("src/owned.rs"),
        "//! Owned module.\npub fn public_api() {}\n",
    )
    .expect("write owned module");

    let output = run_cli([root.as_os_str()]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("[advice]"), "{stdout}");
    assert!(stdout.contains("AGENT-R002"), "{stdout}");
}

#[test]
fn cli_exits_nonzero_for_blocking_findings() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-blocking");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)] mod tests { #[test] fn it_works() {} }\n",
    )
    .expect("write lib");

    let output = run_cli([root.as_os_str()]);

    assert!(!output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.starts_with("[lint:warning]"), "{stdout}");
    assert!(stdout.contains("RUST-PROJ-R003"), "{stdout}");
}

fn run_cli<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    Command::new(env!("CARGO_BIN_EXE_rust-project-harness"))
        .args(args)
        .output()
        .expect("run cli")
}

fn write_manifest(root: &Path, name: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("write manifest");
}

fn write_clean_source(root: &Path) {
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
}
