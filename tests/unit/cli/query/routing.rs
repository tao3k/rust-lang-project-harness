use tempfile::TempDir;

use crate::cli::support::{
    normalize_temp_root, run_cli, write_clean_source, write_complex_dependency_fixture,
    write_manifest,
};

#[test]
fn cli_query_terms_require_asp_lexical_workspace_search() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_complex_dependency_fixture(root);
    let output = run_cli([
        "query".as_ref(),
        "--term".as_ref(),
        "RuntimeClient".as_ref(),
        "--term".as_ref(),
        "send_bytes".as_ref(),
        "--surface".as_ref(),
        "tests".as_ref(),
        "--view".as_ref(),
        "seeds".as_ref(),
        root.as_os_str(),
    ]);
    assert!(!output.status.success(), "{output:?}");
    let stderr = normalize_temp_root(
        &String::from_utf8(output.stderr).expect("utf8 stderr"),
        root,
    );
    assert!(
        stderr.contains("query workspace term discovery is owned by ASP search lexical"),
        "{stderr}"
    );
    assert!(
        stderr.contains(
            "asp rust search lexical 'RuntimeClient send_bytes' owner tests --workspace <workspace-root> --view seeds"
        ),
        "{stderr}"
    );
}

#[test]
fn cli_query_broad_glob_selector_routes_to_prime_seeds() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-query-glob");
    write_clean_source(root);
    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "bulk-source-dump".as_ref(),
        "--selector".as_ref(),
        "**/*.rs".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(
        stdout.starts_with("[search-prime] root=. alg=budgeted-prime-frontier-v1"),
        "{stdout}"
    );
    assert!(
        stdout.contains("O=owner:path(src/lib.rs)!owner"),
        "{stdout}"
    );
    assert!(!stdout.contains("G>{O:selects}"), "{stdout}");
    assert!(!stdout.contains("rank=O frontier=O.owner"), "{stdout}");
    assert!(stdout.contains("frontier ID.next"), "{stdout}");
    assert!(
        stdout.contains("entries=owner-tests(O=>covering-tests+test-entrypoints+fixtures)"),
        "{stdout}"
    );
    assert!(!stdout.contains("|seed "), "{stdout}");
}
