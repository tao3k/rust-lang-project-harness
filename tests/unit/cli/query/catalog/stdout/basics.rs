use tempfile::TempDir;

use crate::cli::query::catalog::function_name_query_args;
use crate::cli::support::run_cli;

#[test]
fn tree_sitter_query_stdout_renders_capture_frontier_graph_without_code() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn exposed() -> usize {\n    1\n}\n\nstruct Hidden;\n",
    )
    .expect("fixture");

    let output = run_cli(function_name_query_args(root, &[]));
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert!(stdout.starts_with("[query-treesitter] root="), "{stdout}");
    assert!(
        stdout.contains(
            "lang=rust pattern=function_item/name capture=function.name alg=syntax-capture-frontier"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("Q=tsquery:pattern(function_item/name)!query"),
        "{stdout}"
    );
    assert!(
        stdout.contains("C=capture:function.name(exposed)@src/lib.rs:1!code ts=identifier/name"),
        "{stdout}"
    );
    assert!(
        stdout.contains("I=item:fn(exposed)@src/lib.rs:1:3!code ts=function_item"),
        "{stdout}"
    );
    assert!(stdout.contains("C>{I:enclosing-item}"), "{stdout}");
    assert!(stdout.contains("rank=I"), "{stdout}");
    assert!(stdout.contains("frontier=I.code"), "{stdout}");
    assert!(!stdout.contains("pub fn exposed()"));
    assert!(!stdout.contains("----"));
    assert!(!stdout.contains("|syntax-query"));
    assert!(!stdout.contains("|syntax-capture"));
    assert!(!stdout.contains("artifactId="));
    assert!(!stdout.contains("clientDbHint="));
    assert!(!stdout.contains("matches=0"));
}

#[test]
fn tree_sitter_query_frontier_output_can_be_filtered_by_term() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn alpha() -> usize {\n    1\n}\n\npub fn beta_target() -> usize {\n    2\n}\n",
    )
    .expect("fixture");

    let output = run_cli(function_name_query_args(root, &["--term", "beta"]));
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert!(
        stdout
            .contains("C=capture:function.name(beta_target)@src/lib.rs:5!code ts=identifier/name"),
        "{stdout}"
    );
    assert!(
        stdout.contains("I=item:fn(beta_target)@src/lib.rs:5:7!code ts=function_item"),
        "{stdout}"
    );
    assert!(stdout.contains("frontier=I.code"), "{stdout}");
    assert!(!stdout.contains("name=alpha"));
    assert!(!stdout.contains("pub fn"));
    assert!(!stdout.contains("----"));
    assert!(!stdout.contains("|syntax-query"));
    assert!(!stdout.contains("|syntax-capture"));
    assert!(!stdout.contains("artifactId="));
}

#[test]
fn tree_sitter_query_frontier_output_reports_overflow_cap() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    let source = (0..85)
        .map(|index| format!("pub fn generated_{index}() -> usize {{\n    {index}\n}}\n"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(root.join("src/lib.rs"), source).expect("fixture");

    let output = run_cli(function_name_query_args(root, &[]));
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert!(
        stdout.contains("matches=85 shown=12 omitted=73"),
        "{stdout}"
    );
    assert!(
        stdout.contains("omit=code,full-node-list,overflow-captures"),
        "{stdout}"
    );
    assert!(stdout.contains("frontier=I.code,I2.code"), "{stdout}");
    assert!(stdout.contains("I12=item:fn(generated_11)"), "{stdout}");
    assert!(!stdout.contains("I13=item:fn(generated_12)"), "{stdout}");
}
