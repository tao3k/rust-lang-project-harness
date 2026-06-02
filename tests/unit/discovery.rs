use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::discovery::{
    discover_cargo_package_roots, glob_pattern_matches, rust_project_harness_scope,
};

#[test]
fn workspace_member_glob_matches_forward_slash_relative_paths() {
    assert!(glob_pattern_matches("crates/*", "crates/member"));
    assert!(!glob_pattern_matches("crates/*", "crates/member/nested"));
}

#[test]
fn workspace_member_glob_matches_windows_relative_paths() {
    assert!(glob_pattern_matches("crates/*", r"crates\member"));
    assert!(!glob_pattern_matches("crates/*", r"crates\member\nested"));
}

#[test]
fn project_scope_is_anchored_to_cargo_manifest_targets() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"cargo-anchored\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"custom/lib.rs\"\n\n[[bin]]\nname = \"worker\"\npath = \"tools/worker.rs\"\n\n[[test]]\nname = \"contract\"\npath = \"contracts/api.rs\"\n\n[[example]]\nname = \"demo\"\npath = \"demo/example.rs\"\n\n[[bench]]\nname = \"throughput\"\npath = \"perf/throughput.rs\"\nharness = false\n",
    )
    .expect("write manifest");
    write_file(root, "src/lib.rs");
    write_file(root, "tests/smoke.rs");
    write_file(root, "custom/lib.rs");
    write_file(root, "tools/worker.rs");
    write_file(root, "contracts/api.rs");
    write_file(root, "demo/example.rs");
    write_file(root, "perf/throughput.rs");

    let scope = rust_project_harness_scope(root, true, &[], &[]);
    let source_paths = path_set(root, &scope.source_paths);
    let test_paths = path_set(root, &scope.test_paths);
    let package_paths = path_set(root, &scope.package_paths);

    assert!(source_paths.contains("src"));
    assert!(source_paths.contains("custom"));
    assert!(source_paths.contains("tools"));
    assert!(test_paths.contains("tests"));
    assert!(test_paths.contains("contracts"));
    assert!(package_paths.contains("demo/example.rs"));
    assert!(package_paths.contains("perf/throughput.rs"));
}

#[test]
fn workspace_path_dependencies_are_discovered_as_package_roots() {
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
    write_file(root, "crates/hook/src/lib.rs");
    write_file(root, "languages/rust-lang-project-harness/src/lib.rs");

    let package_roots = discover_cargo_package_roots(root, &BTreeSet::new());
    let package_roots = path_set(root, &package_roots);

    assert!(package_roots.contains("crates/hook"));
    assert!(package_roots.contains("languages/rust-lang-project-harness"));
}

fn write_file(root: &Path, relative_path: &str) {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, "//! Fixture.\n").expect("write file");
}

fn path_set(root: &Path, paths: &[PathBuf]) -> BTreeSet<String> {
    paths
        .iter()
        .map(|path| {
            path.strip_prefix(root)
                .expect("project relative path")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}
