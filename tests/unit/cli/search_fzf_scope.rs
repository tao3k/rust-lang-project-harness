use super::support::run_search;

#[test]
fn cli_search_fzf_accepts_workspace_relative_scope_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("workspace").join("crates").join("demo");
    std::fs::create_dir_all(root.join("src")).expect("src");
    std::fs::create_dir_all(root.join("tests/unit")).expect("tests");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "pub fn runCodexAgentHook() {}\n").expect("src");
    std::fs::write(
        root.join("tests/unit/snapshot.rs"),
        "fn snapshot() { runCodexAgentHook(); }\n",
    )
    .expect("test");

    let output = run_search(
        &root,
        &[
            "fzf",
            "runCodexAgentHook",
            "owner",
            "crates/demo/tests",
            "--view",
            "seeds",
        ],
    );

    assert!(
        output.starts_with(
            "[search-fzf] q=runCodexAgentHook scope=crates/demo/tests alg=seed-frontier"
        ),
        "{output}"
    );
    assert!(
        output.contains("T=test:path(tests/unit/snapshot.rs)!tests"),
        "{output}"
    );
    assert!(!output.contains("src/lib.rs"), "{output}");

    let token_set_output = run_search(
        &root,
        &[
            "fzf",
            "missing_token runCodexAgentHook",
            "owner",
            "crates/demo/tests",
            "--view",
            "seeds",
        ],
    );

    assert!(
        token_set_output.contains("querySet=2"),
        "{token_set_output}"
    );
    assert!(
        token_set_output.contains("tests/unit/snapshot.rs"),
        "{token_set_output}"
    );
}
