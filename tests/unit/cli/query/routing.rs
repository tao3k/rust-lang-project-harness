use tempfile::TempDir;

use crate::cli::support::{
    normalize_temp_root, run_cli, write_clean_source, write_complex_dependency_fixture,
    write_manifest,
};

#[test]
fn cli_query_terms_route_to_fzf_query_set_seeds() {
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
    assert!(output.status.success(), "{output:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(
        stdout.starts_with("[search-fzf] q=RuntimeClient,send_bytes querySet=2 selector=fuzzy-set alg=change-frontier-query-set"),
        "{stdout}"
    );
    assert!(
        stdout.contains("Q=query:term(RuntimeClient,send_bytes)!fzf"),
        "{stdout}"
    );
    assert!(
        stdout.contains("owner:path(src/http/client.rs)!owner"),
        "{stdout}"
    );
    assert!(stdout.contains("owner:path(src/lib.rs)!owner"), "{stdout}");
    assert!(
        stdout.contains("T=test:path(tests/flow.rs)!tests"),
        "{stdout}"
    );
    assert!(
        stdout.contains("rank=Q,O,O2,T frontier=Q.fzf,O.owner,O2.owner,T.tests"),
        "{stdout}"
    );
    assert!(
        stdout.contains("avoid=broad-fzf,raw-read,repeat-glob"),
        "{stdout}"
    );
    assert!(!stdout.contains("|seed "), "{stdout}");
    assert!(stdout.contains("legend:"), "{stdout}");
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
