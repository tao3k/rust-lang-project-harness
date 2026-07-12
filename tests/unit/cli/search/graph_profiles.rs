use std::fs;

use tempfile::TempDir;

use crate::cli::support::run_search;

#[test]
fn cli_search_graph_profiles_filter_to_rendered_aliases() {
    if crate::cli::support::skip_if_protocol_graph_renderer_unavailable() {
        return;
    }

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

    let output = run_search(root, &["symbol", "run_codex_agent_hook", "--view", "seeds"]);

    assert!(output.contains("graph:{G=search,O=owner}"), "{output}");
    assert!(output.contains("owner:path("), "{output}");
    assert!(output.contains("src/lib.rs"), "{output}");
    assert!(output.contains("tests/unit/snapshot.rs"), "{output}");
    assert!(
        output.contains("q=run_codex_agent_hook") || output.contains("run_codex_agent_hook"),
        "{output}"
    );
    assert!(!output.contains("search-lexical"), "{output}");
}
