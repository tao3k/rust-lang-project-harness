use tempfile::TempDir;

use crate::cli::support::run_cli;

#[test]
fn tree_sitter_query_predicate_filters_capture_text() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn alpha() -> usize {\n    1\n}\n\npub fn beta_target() -> usize {\n    2\n}\n",
    )
    .expect("fixture");

    let output = run_cli([
        "query".as_ref(),
        "--treesitter-query".as_ref(),
        r#"(function_item name: (identifier) @function.name (#eq? @function.name "beta_target"))"#
            .as_ref(),
        root.as_os_str(),
        "--asp-syntax-query-captures".as_ref(),
        "function.definition,function.name".as_ref(),
        "--asp-syntax-query-node-types".as_ref(),
        "function_item,identifier".as_ref(),
        "--asp-syntax-query-fields".as_ref(),
        "name".as_ref(),
        "--asp-syntax-query-predicates-json".as_ref(),
        r#"[{"op":"eq","capture":"function.name","values":[{"kind":"string","value":"beta_target"}]}]"#
            .as_ref(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert!(
        stdout.contains("I=item:fn(beta_target)@src/lib.rs:5:7!code ts=function_item"),
        "{stdout}"
    );
    assert!(!stdout.contains("I=item:fn(alpha)"), "{stdout}");
    assert!(!stdout.contains("pub fn"));
}

#[test]
fn tree_sitter_query_predicate_prefilter_skips_deep_irrelevant_sources() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(root.join("src/lib.rs"), "pub fn needle_target() {}\n").expect("target fixture");

    let mut deep_irrelevant = String::new();
    for index in 0..4096 {
        deep_irrelevant.push_str(&format!("mod irrelevant_{index} {{\n"));
    }
    deep_irrelevant.push_str("pub fn unrelated() {}\n");
    for _ in 0..4096 {
        deep_irrelevant.push_str("}\n");
    }
    std::fs::write(root.join("src/deep_irrelevant.rs"), deep_irrelevant).expect("deep fixture");

    let output = run_cli([
        "query".as_ref(),
        "--treesitter-query".as_ref(),
        r#"(function_item name: (identifier) @function.name (#eq? @function.name "needle_target"))"#
            .as_ref(),
        root.as_os_str(),
        "--asp-syntax-query-captures".as_ref(),
        "function.name".as_ref(),
        "--asp-syntax-query-node-types".as_ref(),
        "function_item,identifier".as_ref(),
        "--asp-syntax-query-fields".as_ref(),
        "name".as_ref(),
        "--asp-syntax-query-predicates-json".as_ref(),
        r#"[{"op":"eq","capture":"function.name","values":[{"kind":"string","value":"needle_target"}]}]"#
            .as_ref(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert!(
        stdout.contains("I=item:fn(needle_target)@src/lib.rs:1!code ts=function_item"),
        "{stdout}"
    );
    assert!(!stdout.contains("deep_irrelevant"), "{stdout}");
}

#[test]
fn tree_sitter_query_match_predicate_filters_capture_text() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn alpha() {}\n\npub fn beta_target() {}\n",
    )
    .expect("fixture");

    let output = run_cli([
        "query".as_ref(),
        "--treesitter-query".as_ref(),
        r#"(function_item name: (identifier) @function.name (#match? @function.name "^beta"))"#
            .as_ref(),
        root.as_os_str(),
        "--asp-syntax-query-captures".as_ref(),
        "function.name".as_ref(),
        "--asp-syntax-query-node-types".as_ref(),
        "function_item,identifier".as_ref(),
        "--asp-syntax-query-fields".as_ref(),
        "name".as_ref(),
        "--asp-syntax-query-predicates-json".as_ref(),
        r#"[{"op":"match","capture":"function.name","values":[{"kind":"string","value":"^beta"}]}]"#
            .as_ref(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert!(
        stdout.contains("I=item:fn(beta_target)@src/lib.rs:3!code ts=function_item"),
        "{stdout}"
    );
    assert!(!stdout.contains("I=item:fn(alpha)"), "{stdout}");
    assert!(!stdout.contains("|syntax-query-unsupported"), "{stdout}");
}

#[test]
fn tree_sitter_query_any_eq_predicate_filters_capture_text() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn alpha() {}\n\npub fn beta_target() {}\n",
    )
    .expect("fixture");

    let output = run_cli([
        "query".as_ref(),
        "--treesitter-query".as_ref(),
        r#"(function_item name: (identifier) @function.name (#any-eq? @function.name "beta_target"))"#
            .as_ref(),
        root.as_os_str(),
        "--asp-syntax-query-captures".as_ref(),
        "function.name".as_ref(),
        "--asp-syntax-query-node-types".as_ref(),
        "function_item,identifier".as_ref(),
        "--asp-syntax-query-fields".as_ref(),
        "name".as_ref(),
        "--asp-syntax-query-predicates-json".as_ref(),
        r#"[{"op":"any-eq","capture":"function.name","values":[{"kind":"string","value":"beta_target"}]}]"#
            .as_ref(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert!(
        stdout.contains("I=item:fn(beta_target)@src/lib.rs:3!code ts=function_item"),
        "{stdout}"
    );
    assert!(!stdout.contains("I=item:fn(alpha)"), "{stdout}");
    assert!(!stdout.contains("|syntax-query-unsupported"), "{stdout}");
}

#[test]
fn tree_sitter_query_any_match_predicate_filters_capture_text() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn alpha() {}\n\npub fn beta_target() {}\n",
    )
    .expect("fixture");

    let output = run_cli([
        "query".as_ref(),
        "--treesitter-query".as_ref(),
        r#"(function_item name: (identifier) @function.name (#any-match? @function.name "^beta"))"#
            .as_ref(),
        root.as_os_str(),
        "--asp-syntax-query-captures".as_ref(),
        "function.name".as_ref(),
        "--asp-syntax-query-node-types".as_ref(),
        "function_item,identifier".as_ref(),
        "--asp-syntax-query-fields".as_ref(),
        "name".as_ref(),
        "--asp-syntax-query-predicates-json".as_ref(),
        r#"[{"op":"any-match","capture":"function.name","values":[{"kind":"string","value":"^beta"}]}]"#
            .as_ref(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert!(
        stdout.contains("I=item:fn(beta_target)@src/lib.rs:3!code ts=function_item"),
        "{stdout}"
    );
    assert!(!stdout.contains("I=item:fn(alpha)"), "{stdout}");
    assert!(!stdout.contains("|syntax-query-unsupported"), "{stdout}");
}

#[test]
fn tree_sitter_query_not_predicate_filters_capture_text() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn alpha() {}\n\npub fn beta_target() {}\n",
    )
    .expect("fixture");

    let output = run_cli([
        "query".as_ref(),
        "--treesitter-query".as_ref(),
        r#"(function_item name: (identifier) @function.name (#not-eq? @function.name "alpha"))"#
            .as_ref(),
        root.as_os_str(),
        "--asp-syntax-query-captures".as_ref(),
        "function.name".as_ref(),
        "--asp-syntax-query-node-types".as_ref(),
        "function_item,identifier".as_ref(),
        "--asp-syntax-query-fields".as_ref(),
        "name".as_ref(),
        "--asp-syntax-query-predicates-json".as_ref(),
        r#"[{"op":"not-eq","capture":"function.name","values":[{"kind":"string","value":"alpha"}]}]"#
            .as_ref(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert!(
        stdout.contains("I=item:fn(beta_target)@src/lib.rs:3!code ts=function_item"),
        "{stdout}"
    );
    assert!(!stdout.contains("I=item:fn(alpha)"), "{stdout}");
}
