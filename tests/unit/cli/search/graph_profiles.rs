use std::fs;

use tempfile::TempDir;

use crate::cli::support::run_search;

#[test]
fn cli_search_graph_profiles_filter_to_rendered_aliases() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"graph-profiles\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "pub fn run_codex_agent_hook() {}\npub fn unrelated() {}\n",
    )
    .expect("write lib");
    fs::create_dir_all(root.join("tests/unit")).expect("create tests");
    fs::write(
        root.join("tests/unit/snapshot.rs"),
        "use graph_profiles::run_codex_agent_hook;\n#[test]\nfn snapshot() { run_codex_agent_hook(); }\n",
    )
    .expect("write test");

    let output = run_search(
        root,
        &[
            "fzf",
            "run_codex_agent_hook",
            "owner",
            "src",
            "tests/unit",
            "--view",
            "seeds",
        ],
    );

    assert!(
        output.contains("O=owner:path(src/lib.rs)!owner"),
        "{output}"
    );
    assert!(
        output.contains("T=test:path(tests/unit/snapshot.rs)!tests"),
        "{output}"
    );
    assert!(
        output.contains("Q=query:term(run_codex_agent_hook)!fzf"),
        "{output}"
    );
    assert!(
        output.contains(
            "entries=owner-query(O,Q=>items+tests+dependency-usage),owner-tests(O=>covering-tests+test-entrypoints+fixtures)"
        ),
        "{output}"
    );
    assert!(!output.contains("query-deps("), "{output}");
    assert!(!output.contains("finding-frontier("), "{output}");
}
