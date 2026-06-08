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
    assert!(
        stdout.starts_with("[search-guide] protocol=search-guide.v1"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|catalog reasoningProfiles=owner-query,query-deps,owner-tests,finding-frontier,feature-cfg entries=owner-query,query-deps,owner-tests,finding-frontier,feature-cfg routes=path,read-frontier"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("  overview-prime:\n    command=search prime --view seeds"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "  owner-query:\n    command=search reasoning owner-query --owner <owner-path> --query <term> --view seeds\n    args=owner:path query:term"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "    command=search reasoning owner-query --owner <owner-path> --query <term> --view seeds"
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("owner-items"), "{stdout}");
    assert!(
        stdout.contains(
            "  query-deps:\n    command=search reasoning query-deps --query <term> --dependency <pkg> --view seeds\n    args=query:term dependency:pkg"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "    command=search reasoning query-deps --query <term> --dependency <pkg> --view seeds"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "  finding-frontier:\n    command=search reasoning finding-frontier --query <finding-term> --owner <owner-path> --view seeds\n    args=finding:term owner:path"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "    command=search reasoning finding-frontier --query <finding-term> --owner <owner-path> --view seeds"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "  feature-cfg:\n    command=search reasoning feature-cfg --query <feature-name> --view seeds\n    args=feature:name"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "    command=search reasoning feature-cfg --query <feature-name> --view seeds"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("  read-frontier:\n    args=range:path@start:end"),
        "{stdout}"
    );
    assert!(
        stdout.contains("avoid=raw-read,manual-window-scan,full-json,natural-language-intent"),
        "{stdout}"
    );
    assert!(!stdout.contains("|entry "), "{stdout}");
    assert!(!stdout.contains("|route "), "{stdout}");
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
