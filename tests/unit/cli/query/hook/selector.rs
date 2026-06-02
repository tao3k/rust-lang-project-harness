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
            "|query itemQuery=load status=hit match=exact item=1 reason=parser-item-exact next=code"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("|code path=src/lib.rs"), "{stdout}");
}
