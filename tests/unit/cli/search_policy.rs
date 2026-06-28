use serde_json::Value;
use tempfile::TempDir;

use super::support::{run_cli, write_manifest};

#[test]
fn cli_search_policy_renders_semantic_handles() {
    if crate::cli::support::skip_if_protocol_graph_renderer_unavailable() {
        return;
    }

    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-search-policy");

    let seeds = run_cli([
        "search".as_ref(),
        "policy".as_ref(),
        "RUST-AGENT-PROJECT-001".as_ref(),
        "owner".as_ref(),
        "tests".as_ref(),
        "--view".as_ref(),
        "seeds".as_ref(),
        root.as_os_str(),
    ]);
    assert!(seeds.status.success(), "{seeds:?}");
    let stdout = String::from_utf8(seeds.stdout).expect("utf8 stdout");
    assert!(
        stdout.starts_with("[search-policy] q=RUST-AGENT-PROJECT-001 alg=policy-handle-catalog"),
        "{stdout}"
    );
    assert!(
        stdout.contains("O=owner:path(src/rules/project_policy/pack.rs)!owner"),
        "{stdout}"
    );
    assert!(
        stdout.contains("T=test:path(tests/unit/rule_catalog.rs)!tests"),
        "{stdout}"
    );
    assert!(stdout.contains("frontier=O.owner"), "{stdout}");
    assert!(!stdout.contains("|seed "), "{stdout}");
    assert!(!stdout.contains("|synthesis "), "{stdout}");
    assert!(
        stdout.contains("tests/unit/path_policy/project/build_gate.rs"),
        "{stdout}"
    );

    let compact = run_cli([
        "search".as_ref(),
        "policy".as_ref(),
        "branch-module".as_ref(),
        "owner".as_ref(),
        "tests".as_ref(),
        root.as_os_str(),
    ]);
    assert!(compact.status.success(), "{compact:?}");
    let stdout = String::from_utf8(compact.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("|handle RUST-AGENT-DOCS-BRANCH-008 kind=policy-rule"),
        "{stdout}"
    );
    assert!(
        stdout.contains("title=Branch_module_lacks_reasoning-tree_intent_doc"),
        "{stdout}"
    );

    let json = run_cli([
        "search".as_ref(),
        "policy".as_ref(),
        "RUST-AGENT-DOCS-BRANCH-008".as_ref(),
        "owner".as_ref(),
        "tests".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(json.status.success(), "{json:?}");
    let packet = serde_json::from_slice::<Value>(&json.stdout).expect("policy json");
    assert_eq!(packet["view"], "policy");
    assert_eq!(
        packet["semanticHandles"][0]["id"],
        "RUST-AGENT-DOCS-BRANCH-008"
    );
    assert_eq!(
        packet["semanticHandles"][0]["ownerPath"],
        "src/rules/agent_policy/pack.rs"
    );
    assert!(
        packet["semanticHandles"][0]["testPaths"]
            .as_array()
            .expect("test paths")
            .iter()
            .any(|path| path.as_str() == Some("tests/unit/policy_contract.rs"))
    );
}
