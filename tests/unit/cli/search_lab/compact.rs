use std::fs;

use tempfile::TempDir;

use crate::cli::support::{run_search, write_manifest};

#[test]
fn search_lab_compacts_prime_cfg_and_edge_rows() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "cli-search-prime-compact"
version = "0.1.0"
edition = "2021"

[features]
fs = []
io-util = []
io-uring = []
"#,
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src/cli")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        r#"pub mod agent_snapshot;
pub mod build_gate;
pub mod cli;

#[cfg(feature = "fs")]
pub mod fs_feature {}
#[cfg(feature = "io-util")]
pub mod io_util_feature {}
#[cfg(feature = "io-uring")]
pub mod io_uring_feature {}
"#,
    )
    .expect("write lib");
    fs::write(root.join("src/agent_snapshot.rs"), "pub fn snapshot() {}\n")
        .expect("write agent snapshot");
    fs::write(root.join("src/build_gate.rs"), "pub fn gate() {}\n").expect("write build gate");
    fs::write(root.join("src/cli/mod.rs"), "pub fn run() {}\n").expect("write cli");

    let stdout = run_search(root, &["prime"]);

    assert!(
        stdout
            .contains("|cfg feature:{fs,io-util,io-uring} next=cfg:feature:{fs,io-util,io-uring}"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|edge O:src/lib.rs -mod-> O:src/{agent_snapshot.rs,build_gate.rs,cli/mod.rs}"
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("|edge O:src/lib.rs -mod-> O:src/agent_snapshot.rs\n"));
}

#[test]
fn search_lab_compacts_repeated_rows_across_non_prime_views() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-search-compact-all");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::create_dir_all(root.join("tests")).expect("create tests");
    fs::write(root.join("src/lib.rs"), "pub fn load() -> usize { 1 }\n").expect("write lib");
    for test in ["alpha", "beta", "gamma"] {
        fs::write(
            root.join(format!("tests/{test}.rs")),
            "use cli_search_compact_all::load;\n#[test]\nfn uses_load() { assert_eq!(load(), 1); }\n",
        )
        .expect("write test");
    }

    let stdout = run_search(root, &["tests", "src/lib.rs"]);

    assert!(
        stdout.contains("|edge O:src/lib.rs -test-> T:tests/{alpha.rs,beta.rs,gamma.rs}"),
        "{stdout}"
    );
    assert!(!stdout.contains("|edge O:src/lib.rs -test-> T:tests/alpha.rs\n"));
}
