use tempfile::TempDir;

use crate::cli::support::{run_cli, write_clean_source, write_manifest};

#[test]
fn cli_agent_guide_advertises_query_reroute() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-query-guide");
    write_clean_source(root);
    let output = run_cli(["agent".as_ref(), "guide".as_ref(), root.as_os_str()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("asp rust query <path> --query <symbol-or-a|b|c> ."),
        "{stdout}"
    );
    assert!(
        stdout.contains("asp rust query <path> --query <symbol-or-a|b|c> --code ."),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|route syntax-locate selectors=S:tree-sitter-query,R:range returns=locator,capture,frontier code=false"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|route syntax-code selectors=S:tree-sitter-query,R:exact-selector|unique-predicate returns=code code=pure multi-match=deny"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|rule syntax predicates supported=#eq?,#any-eq?,#any-of?,#match?,#any-match?,#not-eq?,#not-match? unsupported=none unsupportedReported=true"
        ),
        "{stdout}"
    );
}
