#![allow(unused_imports)]

use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::cli::support::{
    normalize_temp_root, run_cli, run_search, run_search_with_stdin, write_manifest,
    write_search_fixture,
};

#[test]
fn cli_search_prime_renders_line_protocol() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-search-prime");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod domain;\nuse crate::domain::Thing;\npub fn load() -> Thing { Thing }\n",
    )
    .expect("write lib");
    fs::write(root.join("src/hook_runtime.rs"), "pub fn execute() {}\n").expect("write path owner");
    fs::write(
        root.join("src/domain/mod.rs"),
        "//! Domain branch.\npub struct Thing;\n",
    )
    .expect("write domain");

    let output = run_cli(["search".as_ref(), "prime".as_ref(), root.as_os_str()]);

    assert!(output.status.success(), "{output:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(
        stdout.starts_with("[search-prime] mode=package package=."),
        "{stdout}"
    );
    assert!(stdout.contains("|package ."), "{stdout}");
    assert!(
        stdout.contains("|decision purpose=decision-primer"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "capabilities=pipe,fzf,fd-query,rg-query,owner-items,selector-code,treesitter-query"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("ladder=pipe>fzf>fd-query|rg-query>owner-items>selector-code"),
        "{stdout}"
    );
    assert!(
        stdout.contains("history=asp-artifacts:directReadRisk,repeatedPrime,repeatedPipe,bestPath"),
        "{stdout}"
    );
    assert!(
        stdout.contains("risk=broad-direct-read,manual-window-scan,repeat-prime"),
        "{stdout}"
    );
    assert!(
        stdout
            .contains("next=\"asp rust search pipe '<question-or-feature-term>' --view seeds .\""),
        "{stdout}"
    );
    assert!(stdout.contains("|owner src/lib.rs"), "{stdout}");
    assert!(
        stdout.contains("|edge O:src/lib.rs -mod-> O:src/domain/mod.rs"),
        "{stdout}"
    );
    assert!(
        stdout.contains("|edge O:src/lib.rs -crate:crate-> O:src/domain/mod.rs"),
        "{stdout}"
    );
    assert!(stdout.contains("|next owner:src/lib.rs"), "{stdout}");
    assert!(!stdout.contains("Modules:"), "{stdout}");
    assert!(!stdout.trim_start().starts_with('{'), "{stdout}");
    insta::assert_snapshot!("cli_search_prime", stdout);
}
