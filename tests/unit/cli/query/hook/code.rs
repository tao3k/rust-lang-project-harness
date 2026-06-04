use tempfile::TempDir;

use crate::cli::support::{run_cli, write_search_fixture};

#[test]
fn cli_query_code_output_is_source_slice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);
    let output = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--query".as_ref(),
        "load".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 3, "{stdout}");
    assert!(!stdout.contains("|code"), "{stdout}");
    assert!(!stdout.contains("text="), "{stdout}");
    assert!(stdout.contains("fn load"), "{stdout}");
    assert!(stdout.contains("domain::make_thing()"), "{stdout}");
}
