use tempfile::TempDir;

use crate::cli::support::run_cli;

#[test]
fn tree_sitter_query_calls_catalog_projects_call_frontier() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "fn parse_query() {}\n\nstruct Runner;\nimpl Runner {\n    fn run(&self) {\n        parse_query();\n        self.render_query_local_window();\n        crate::cli::query::print_query_help();\n    }\n}\n",
    )
    .expect("fixture");

    let output = run_cli([
        "query".as_ref(),
        "--catalog".as_ref(),
        "calls".as_ref(),
        "--term".as_ref(),
        "parse_query".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert!(
        stdout.contains(
            "lang=rust pattern=call_expression/function capture=call.expression alg=syntax-capture-frontier"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "C=capture:call.target(parse_query)@src/lib.rs:6!code ts=identifier/function"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("I=item:call(parse_query)@src/lib.rs:5:9!code ts=call_expression"),
        "{stdout}"
    );
    assert!(stdout.contains("frontier=I.code"), "{stdout}");
    assert!(!stdout.contains("fn run"));
}
