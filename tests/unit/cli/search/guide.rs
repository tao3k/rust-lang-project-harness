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
fn cli_search_guide_renders_typed_reasoning_profiles() {
    if crate::cli::support::skip_if_protocol_graph_renderer_unavailable() {
        return;
    }

    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_search_fixture(root);

    let output = run_cli(["search".as_ref(), "guide".as_ref(), root.as_os_str()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("stdout");
    assert!(stdout.starts_with("[search-guide] root="), "{stdout}");
    assert!(stdout.contains("legend:"), "{stdout}");
    assert!(
        stdout.contains("|catalog reasoningProfiles=owner-query,query-deps,owner-tests,finding-frontier,feature-cfg entries=owner-query,query-deps,owner-tests,finding-frontier,feature-cfg routes=read-frontier"),
        "{stdout}"
    );
    assert!(
        stdout.contains("entries=owner-query(O,Q=>items+tests+dependency-usage),query-deps(Q,D=>owners+imports+usage-tests),owner-tests(O=>covering-tests+test-entrypoints+fixtures),finding-frontier(F,O=>affected-owners+tests+verification-actions),feature-cfg(F2=>cfg-gates+owners+verification-surfaces)"),
        "{stdout}"
    );
    assert!(
        stdout.contains("|entry finding-frontier selectors=F:finding,O:owner? returns=affected-owners,tests,verification-actions"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|entry feature-cfg selectors=F2:feature returns=cfg-gates,owners,verification-surfaces"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("|route read-frontier selectors=R:range"),
        "{stdout}"
    );
    assert!(!stdout.contains("compatible="), "{stdout}");
    assert!(!stdout.trim_start().starts_with('{'), "{stdout}");

    let owner_stdout = run_search(
        root,
        &[
            "reasoning",
            "owner-tests",
            "--owner",
            "src/lib.rs",
            "--view",
            "seeds",
        ],
    );
    assert!(
        owner_stdout.starts_with("[search-reasoning] q=owner-tests"),
        "{owner_stdout}"
    );
    assert!(
        owner_stdout.contains("O=owner:path(src/lib.rs)!owner"),
        "{owner_stdout}"
    );
    assert!(
        owner_stdout.contains("entries=owner-tests(O=>covering-tests+test-entrypoints+fixtures)"),
        "{owner_stdout}"
    );

    let owner_packet_stdout = run_search(
        root,
        &[
            "reasoning",
            "owner-tests",
            "--owner",
            "src/lib.rs",
            "--view",
            "seeds",
            "--json",
        ],
    );
    let owner_packet: Value =
        serde_json::from_str(&owner_packet_stdout).expect("owner packet json");
    assert_eq!(owner_packet["view"], "reasoning");
    assert_eq!(owner_packet["renderMode"], "seeds");
    assert_eq!(owner_packet["avoidNextActions"][0]["kind"], "raw-read");
    let profiles = owner_packet["reasoningProfiles"]
        .as_array()
        .expect("profiles");
    let owner_tests_profile = profiles
        .iter()
        .find(|profile| profile["profile"] == "owner-tests")
        .expect("owner-tests profile");
    assert_eq!(owner_tests_profile["selectors"][0]["kind"], "owner");
    assert_eq!(owner_tests_profile["returns"][0], "covering-tests");

    let deps_stdout = run_search(
        root,
        &[
            "reasoning",
            "query-deps",
            "--query",
            "serde",
            "--dependency",
            "serde",
            "--view",
            "seeds",
        ],
    );
    assert!(
        deps_stdout.starts_with("[search-reasoning] q=query-deps"),
        "{deps_stdout}"
    );
    assert!(
        deps_stdout.contains("query-deps(Q,D=>owners+imports+usage-tests)"),
        "{deps_stdout}"
    );

    let feature_stdout = run_search(
        root,
        &[
            "reasoning",
            "feature-cfg",
            "--query",
            "test",
            "--view",
            "seeds",
        ],
    );
    assert!(
        feature_stdout.starts_with("[search-reasoning] q=feature-cfg"),
        "{feature_stdout}"
    );
    assert!(
        feature_stdout.contains("F=feature:feature(test)!cfg"),
        "{feature_stdout}"
    );
    assert!(
        feature_stdout.contains("feature-cfg(F=>cfg-gates+owners+verification-surfaces)"),
        "{feature_stdout}"
    );

    let feature_packet_stdout = run_search(
        root,
        &[
            "reasoning",
            "feature-cfg",
            "--query",
            "test",
            "--view",
            "seeds",
            "--json",
        ],
    );
    let feature_packet: Value =
        serde_json::from_str(&feature_packet_stdout).expect("feature packet json");
    assert!(
        feature_packet["nextActions"]
            .as_array()
            .expect("feature next actions")
            .iter()
            .any(|action| action["kind"] == "feature" && action["target"] == "test")
    );

    let finding_stdout = run_search(
        root,
        &[
            "reasoning",
            "finding-frontier",
            "--query",
            "serde",
            "--owner",
            "src/lib.rs",
            "--view",
            "seeds",
        ],
    );
    assert!(
        finding_stdout.starts_with("[search-reasoning] q=finding-frontier"),
        "{finding_stdout}"
    );
    assert!(
        finding_stdout.contains("F=finding:finding(serde)!finding"),
        "{finding_stdout}"
    );
    assert!(
        finding_stdout
            .contains("finding-frontier(F,O=>affected-owners+tests+verification-actions)"),
        "{finding_stdout}"
    );

    let finding_packet_stdout = run_search(
        root,
        &[
            "reasoning",
            "finding-frontier",
            "--query",
            "serde",
            "--owner",
            "src/lib.rs",
            "--view",
            "seeds",
            "--json",
        ],
    );
    let finding_packet: Value =
        serde_json::from_str(&finding_packet_stdout).expect("finding packet json");
    assert!(
        finding_packet["nextActions"]
            .as_array()
            .expect("finding next actions")
            .iter()
            .any(|action| action["kind"] == "finding" && action["target"] == "serde")
    );
}
