use std::fs;
use std::path::Path;

use tempfile::TempDir;

use crate::cli::support::run_cli;
mod code;
mod selector;

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
        "rust://languages/rust-lang-project-harness/src/lib.rs#item/function/harness".as_ref(),
        "--workspace".as_ref(),
        root.as_os_str(),
        "--code".as_ref(),
        "--json".as_ref(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let value = serde_json::from_slice::<serde_json::Value>(&output.stdout).expect("query json");
    assert_eq!(value["schemaId"], "asp.exact-source-query-result.v1");
    assert_eq!(
        value["resolvedOwnerPath"],
        "languages/rust-lang-project-harness/src/lib.rs"
    );
    assert_eq!(value["itemKind"], "function");
    assert_eq!(value["itemName"], "harness");
    assert_eq!(value["code"], "pub fn harness() {}");
    assert!(matches!(
        (
            value["resolutionEvidence"]["state"].as_str(),
            value["resolutionEvidence"]["authority"].as_str()
        ),
        (Some("live-hit"), Some("live-parser"))
            | (Some("artifact-cache-hit"), Some("content-cache"))
    ));
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
        "--workspace".as_ref(),
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
        "--workspace",
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
        "--workspace".as_ref(),
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
