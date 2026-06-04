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
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_search_fixture(root);

    let output = run_cli(["search".as_ref(), "guide".as_ref(), root.as_os_str()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("stdout");
    assert!(
        stdout.starts_with(
            "[search-guide] language=rust provider=rs-harness protocol=search-guide.v1\n"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|entry owner-query selectors=O:owner,Q:query returns=items,tests,dependency-usage"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "cmd=asp rust search reasoning owner-query --owner <O> --query <Q> --view seeds ."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|entry query-deps selectors=Q:query,D:dependency returns=owners,imports,usage-tests"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|entry owner-tests selectors=O:owner returns=covering-tests,test-entrypoints,fixtures"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|entry read-frontier selectors=R:range returns=symbols,windows,tests,next-actions"
        ),
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
    assert!(owner_stdout.contains("legend:"), "{owner_stdout}");
    assert!(
        owner_stdout.contains("O=owner:path(src/lib.rs)!owner"),
        "{owner_stdout}"
    );
    assert!(owner_stdout.contains("rank="), "{owner_stdout}");
    assert!(owner_stdout.contains("frontier="), "{owner_stdout}");
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
    assert!(deps_stdout.contains("rank="), "{deps_stdout}");
    assert!(deps_stdout.contains("frontier="), "{deps_stdout}");
    assert!(
        deps_stdout.contains("query-deps(Q,D=>owners+imports+usage-tests)"),
        "{deps_stdout}"
    );
}
