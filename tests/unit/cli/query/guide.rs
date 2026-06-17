use tempfile::TempDir;

use crate::cli::support::{run_cli, write_clean_source, write_manifest};

#[test]
fn cli_agent_guide_advertises_query_reroute() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-query-guide");
    write_clean_source(root);
    let output = run_cli(["guide".as_ref(), root.as_os_str()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("[agent-guide] lang=rust provider=asp-rust protocol=agent-guide.v1"),
        "{stdout}"
    );
    assert!(
        stdout.contains("|surface query purpose=locator-or-code output=frontier|pure-code"),
        "{stdout}"
    );
    assert!(
        stdout.contains(r#"|flow bootstrap start="search guide .""#),
        "{stdout}"
    );
    assert!(
        stdout.contains(r#"|refer search-guide="search guide ." use=low-frequency-tool-map"#),
        "{stdout}"
    );
    assert!(
        stdout.contains("|avoid raw-read,manual-window-scan,inline-code-in-search"),
        "{stdout}"
    );
}

#[test]
fn cli_query_help_advertises_dependency_search_surface() {
    let output = run_cli(["query", "--help"]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains(
            "rs-harness search dependency <crate-or-package> [items docs-use tests] [--view seeds] [--workspace WORKSPACE]"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("rs-harness search guide [--workspace WORKSPACE]"),
        "{stdout}"
    );
    assert!(
        stdout.contains("Dependency search is manifest-first"),
        "{stdout}"
    );
}

#[test]
fn cli_query_guide_prints_query_contract() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-query-guide-contract");
    write_clean_source(root);
    let output = run_cli(["query".as_ref(), "guide".as_ref(), root.as_os_str()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("[query-guide] lang=rust provider=asp-rust protocol=query-guide.v1"),
        "{stdout}"
    );
    assert!(
        stdout
            .contains(r#"|contract stdout=frontier unless="--code + exact-selector|unique-match""#),
        "{stdout}"
    );
    assert!(
        stdout.contains(r#"|mode exact-range command="query --from-hook direct-source-read --selector <path:start-end> --code" output=pure-code maxWindow=40"#),
        "{stdout}"
    );
    assert!(
        stdout.contains("|read-plan avoid=repeat-wide-read,manual-window-scan,raw-read"),
        "{stdout}"
    );
}

#[test]
fn cli_query_guide_treesitter_prints_syntax_contract() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-query-guide-treesitter");
    write_clean_source(root);
    let output = run_cli([
        "query".as_ref(),
        "guide".as_ref(),
        "treesitter".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains(
            "[treesitter-query-guide] lang=rust engine=tree-sitter protocol=treesitter-query-guide.v1"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("|contract base=tree-sitter native-extension=rs-harness"),
        "{stdout}"
    );
    assert!(
        stdout.contains(r#"|template id=rust.functions pattern="(function_item name: (identifier) @function.name)""#),
        "{stdout}"
    );
    assert!(
        stdout.contains(r#"|mode exact-code command="query --selector <path-or-range> --treesitter-query <pattern> --workspace <workspace-root> --code" output=pure-code"#),
        "{stdout}"
    );
}
