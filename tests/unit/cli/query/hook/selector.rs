use std::fs;

use tempfile::TempDir;

use crate::cli::support::{normalize_temp_root, run_cli, write_search_fixture};

#[test]
fn cli_query_hook_selector_strips_owner_prefix_and_line_suffix() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);
    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "owner:src/lib.rs:4:8".as_ref(),
        "--query".as_ref(),
        "load".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(
        stdout.starts_with("[search-owner] q=src/lib.rs pkg=. own=1 item=1 itemQuery=load"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|query itemQuery=load status=hit match=exact item=1 reason=parser-item-exact next=query-code"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("|item load kind=fn responsibilities=early-return public=true next=symbol:load read=src/lib.rs:6:6"),
        "{stdout}"
    );
    assert!(!stdout.contains("|code path=src/lib.rs"), "{stdout}");
}

#[test]
fn cli_query_hook_glob_code_shaped_term_uses_compact_frontier() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);
    fs::write(
        root.join("src/types.rs"),
        "pub struct ClientReceipt {\n    pub status: &'static str,\n}\n",
    )
    .expect("write types");

    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "**/*.rs".as_ref(),
        "--term".as_ref(),
        "ClientReceipt {".as_ref(),
        "--surface".as_ref(),
        "owners,tests".as_ref(),
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
        stdout.starts_with("[search-fzf] q=ClientReceipt "),
        "{stdout}"
    );
    assert!(!stdout.contains("q=ClientReceipt {"), "{stdout}");
    assert!(!stdout.contains("querySet=2"), "{stdout}");
    assert!(!stdout.contains("|seed "), "{stdout}");
    assert!(!stdout.contains("|synthesis "), "{stdout}");
    assert!(
        stdout.contains("O=owner:path(src/types.rs)!owner"),
        "{stdout}"
    );
    assert!(stdout.contains("rank=Q,O"), "{stdout}");
    assert!(stdout.contains("frontier=O.owner"), "{stdout}");
    assert!(
        stdout.contains("avoid=broad-fzf,raw-read,repeat-glob"),
        "{stdout}"
    );
}
