use std::fs;
use std::path::Path;

use tempfile::TempDir;

use crate::cli::support::{normalize_temp_root, run_cli, write_search_fixture};

#[test]
fn cli_query_hook_selector_strips_owner_prefix_and_line_suffix() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);
    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "owner:src/lib.rs:4-8".as_ref(),
        "--query".as_ref(),
        "load".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(
        stdout.starts_with("[search-owner] q=src/lib.rs pkg=. own=1 item=1 itemQuery=load"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|query itemQuery=load status=hit match=exact item=1 reason=parser-item-exact next=code"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("|code path=src/lib.rs"), "{stdout}");
}

#[test]
fn cli_query_code_output_is_source_slice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);
    let output = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--query".as_ref(),
        "load".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1, "{stdout}");
    assert!(!stdout.contains("|code"), "{stdout}");
    assert!(!stdout.contains("text="), "{stdout}");
    assert!(
        stdout.contains("pub fn load() -> Thing { domain::make_thing() }"),
        "{stdout}"
    );
    assert!(!stdout.contains("call domain::make_thing"), "{stdout}");
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
    assert!(
        stdout.contains("pub(crate) fn run_search(root: &Path, args: &[&str]) -> String {"),
        "{stdout}"
    );
    assert!(
        stdout.contains("command_args.push(\"search\".into());"),
        "{stdout}"
    );
    assert!(
        stdout.contains("command_args.extend(args.iter().map(std::ffi::OsString::from));"),
        "{stdout}"
    );
    assert!(stdout.contains("normalize_temp_root("), "{stdout}");
    assert!(!stdout.contains("call push"), "{stdout}");
    assert!(!stdout.contains("call extend"), "{stdout}");
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
        "owner:src/lib.rs:4-8".as_ref(),
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
    assert_eq!(value["selector"], "src/lib.rs");
    assert_eq!(value["outputMode"], "read-packet");
    let windows = value["sourceWindows"].as_array().expect("source windows");
    let load_window = windows
        .iter()
        .find(|window| window["itemName"] == "load")
        .expect("load window");
    assert_eq!(load_window["ownerPath"], "src/lib.rs");
    assert!(
        load_window["read"]
            .as_str()
            .expect("read locator")
            .starts_with("src/lib.rs:"),
        "{value}"
    );
}
