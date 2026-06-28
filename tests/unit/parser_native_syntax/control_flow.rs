use std::fs;

use tempfile::TempDir;

use crate::parser::parse_rust_file;

#[test]
fn native_syntax_facts_record_manual_iterator_loop_shapes() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "pub fn summarize(values: &[usize]) -> bool {\n\
         \tlet mut doubled = Vec::new();\n\
         \tfor value in values {\n\
         \t\tif *value > 0 {\n\
         \t\t\tdoubled.push(*value * 2);\n\
         \t\t}\n\
         \t}\n\
         \tfor value in values {\n\
         \t\tif *value > 100 {\n\
         \t\t\treturn true;\n\
         \t\t}\n\
         \t}\n\
         \tlet mut count = 0;\n\
         \tfor value in values {\n\
         \t\tif *value > 10 {\n\
         \t\t\tcount += 1;\n\
         \t\t}\n\
         \t}\n\
         \tlet mut total = 0;\n\
         \tfor value in values {\n\
         \t\ttotal += *value;\n\
         \t}\n\
         \tlet _ = (doubled, count, total);\n\
         \tfalse\n\
         }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let control_flow = module
        .syntax_facts
        .public_function_control_flows
        .into_iter()
        .find(|control_flow| control_flow.function_name == "summarize")
        .expect("summarize control-flow facts");
    assert_eq!(control_flow.manual_collection_loop_count, 1);
    assert_eq!(control_flow.manual_predicate_loop_count, 1);
    assert_eq!(control_flow.manual_count_loop_count, 1);
    assert_eq!(control_flow.manual_numeric_accumulator_loop_count, 1);
    assert_eq!(control_flow.repeated_iterator_source_loop_count, 3);
}

#[test]
fn native_syntax_facts_record_loop_local_linear_membership_scans() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "pub fn select_allowed(rows: &[Row], allowed: &[String]) -> Vec<String> {\n\
         \tlet mut selected = Vec::new();\n\
         \tfor row in rows {\n\
         \t\tif allowed.iter().any(|candidate| candidate == &row.id) {\n\
         \t\t\tselected.push(row.id.clone());\n\
         \t\t}\n\
         \t}\n\
         \tselected\n\
         }\n\
         pub fn select_indexed(rows: &[Row], allowed: &std::collections::BTreeSet<String>) -> Vec<String> {\n\
         \tlet mut selected = Vec::new();\n\
         \tfor row in rows {\n\
         \t\tif allowed.contains(&row.id) {\n\
         \t\t\tselected.push(row.id.clone());\n\
         \t\t}\n\
         \t}\n\
         \tselected\n\
         }\n\
         pub struct Row { pub id: String }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);
    let facts = module.syntax_facts.public_function_control_flows;
    let select_allowed = facts
        .iter()
        .find(|control_flow| control_flow.function_name == "select_allowed")
        .expect("select_allowed facts");
    assert_eq!(select_allowed.linear_membership_scan_loop_count, 1);
    let select_indexed = facts
        .iter()
        .find(|control_flow| control_flow.function_name == "select_indexed")
        .expect("select_indexed facts");
    assert_eq!(select_indexed.linear_membership_scan_loop_count, 0);
}

#[test]
fn native_syntax_facts_record_private_function_and_impl_method_control_flow() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "fn collect_failed(receipt: Receipt) -> Vec<String> {\n\
         \tlet mut failed = Vec::new();\n\
         \tif receipt.failed_query_count > 0 {\n\
         \t\tfor repository in receipt.repositories {\n\
         \t\t\tfor query in repository.query_receipts {\n\
         \t\t\t\tif !query.passed {\n\
         \t\t\t\t\tfailed.push(query.name);\n\
         \t\t\t\t}\n\
         \t\t\t}\n\
         \t\t}\n\
         \t}\n\
         \tfailed\n\
         }\n\
         struct Collector;\n\
         impl Collector {\n\
         \tfn count_failed(&self, receipt: Receipt) -> usize {\n\
         \t\tlet mut count = 0;\n\
         \t\tfor repository in receipt.repositories {\n\
         \t\t\tfor query in repository.query_receipts {\n\
         \t\t\t\tif !query.passed {\n\
         \t\t\t\t\tcount += 1;\n\
         \t\t\t\t}\n\
         \t\t\t}\n\
         \t\t}\n\
         \t\tcount\n\
         \t}\n\
         }\n\
         struct Receipt { failed_query_count: usize, repositories: Vec<Repository> }\n\
         struct Repository { query_receipts: Vec<Query> }\n\
         struct Query { passed: bool, name: String }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let all_control_flows = module.syntax_facts.all_function_control_flows;
    assert_eq!(
        all_control_flows
            .iter()
            .map(|control_flow| control_flow.function_name.as_str())
            .collect::<Vec<_>>(),
        vec!["collect_failed", "count_failed"]
    );
    assert!(module.syntax_facts.public_function_control_flows.is_empty());
    let collect_failed = all_control_flows
        .iter()
        .find(|control_flow| control_flow.function_name == "collect_failed")
        .expect("collect_failed facts");
    assert!(!collect_failed.is_public);
    assert_eq!(collect_failed.max_loop_nesting_depth, 2);
    assert_eq!(collect_failed.max_nesting_depth, 4);
    let count_failed = all_control_flows
        .iter()
        .find(|control_flow| control_flow.function_name == "count_failed")
        .expect("count_failed facts");
    assert!(!count_failed.is_public);
    assert_eq!(count_failed.max_loop_nesting_depth, 2);
}

#[test]
fn native_syntax_facts_ignore_complex_collection_builders() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "fn collect_findings(items: &[usize]) -> Vec<Finding> {\n\
         \tlet mut findings = Vec::new();\n\
         \tfor item in items {\n\
         \t\tfindings.push(Finding::new(*item));\n\
         \t}\n\
         \tfindings\n\
         }\n\
         struct Finding;\n\
         impl Finding {\n\
         \tfn new(_item: usize) -> Self {\n\
         \t\tSelf\n\
         \t}\n\
         }\n\
         fn normalize_path(path: PathBuf) -> PathBuf {\n\
         \tlet mut normalized = PathBuf::new();\n\
         \tfor component in path.components() {\n\
         \t\tnormalized.push(component.as_os_str());\n\
         \t}\n\
         \tnormalized\n\
         }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let collect_findings = module
        .syntax_facts
        .all_function_control_flows
        .iter()
        .find(|control_flow| control_flow.function_name == "collect_findings")
        .expect("collect_findings facts");
    assert_eq!(collect_findings.manual_collection_loop_count, 0);
    let normalize_path = module
        .syntax_facts
        .all_function_control_flows
        .iter()
        .find(|control_flow| control_flow.function_name == "normalize_path")
        .expect("normalize_path facts");
    assert_eq!(normalize_path.manual_collection_loop_count, 0);
}

#[test]
fn native_syntax_facts_record_literal_dispatch_chains() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "pub fn route(kind: &str) -> usize {\n\
         \tif kind == \"alpha\" {\n\
         \t\t1\n\
         \t} else if kind == \"beta\" {\n\
         \t\t2\n\
         \t} else if kind == \"gamma\" {\n\
         \t\t3\n\
         \t} else {\n\
         \t\t0\n\
         \t}\n\
         }\n\
         pub fn route_method(kind: String) -> usize {\n\
         \tif kind.as_str() == \"alpha\" {\n\
         \t\t1\n\
         \t} else if kind.as_str() == \"beta\" {\n\
         \t\t2\n\
         \t} else if kind.as_str() == \"gamma\" {\n\
         \t\t3\n\
         \t} else {\n\
         \t\t0\n\
         \t}\n\
         }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);
    let dispatch_counts = module
        .syntax_facts
        .public_function_control_flows
        .iter()
        .map(|control_flow| {
            (
                control_flow.function_name.as_str(),
                control_flow.literal_dispatch_chain_count,
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(dispatch_counts, vec![("route", 1), ("route_method", 1)]);
}

#[test]
fn native_syntax_facts_record_repeated_iterator_method_sources() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "pub fn score(values: &[usize]) -> usize {\n\
         \tlet mut total = 0;\n\
         \tfor value in values.iter() {\n\
         \t\ttotal += *value;\n\
         \t}\n\
         \tfor value in values.iter() {\n\
         \t\tif *value > 10 {\n\
         \t\t\ttotal += 1;\n\
         \t\t}\n\
         \t}\n\
         \ttotal\n\
         }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let control_flow = module
        .syntax_facts
        .public_function_control_flows
        .into_iter()
        .find(|control_flow| control_flow.function_name == "score")
        .expect("score control-flow facts");
    assert_eq!(control_flow.repeated_iterator_source_loop_count, 1);
}

#[test]
fn native_syntax_facts_record_public_function_control_flow() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "pub fn classify(kind: &str, rows: &[u8]) -> usize {\n\
         \tlet mut total = 0;\n\
         \tfor row in rows {\n\
         \t\tif *row > 0 {\n\
         \t\t\tmatch kind {\n\
         \t\t\t\t\"alpha\" => total += 1,\n\
         \t\t\t\t\"beta\" => total += 2,\n\
         \t\t\t\t_ => total += 3,\n\
         \t\t\t}\n\
         \t\t}\n\
         \t}\n\
         \ttotal\n\
         }\n\
         fn private_algorithm() {\n\
         \tif true {}\n\
         }\n\
         #[cfg(test)]\n\
         pub fn fixture_algorithm() {\n\
         \tif true {}\n\
         }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let control_flows = module.syntax_facts.public_function_control_flows;
    assert_eq!(
        control_flows
            .iter()
            .map(|control_flow| control_flow.function_name.as_str())
            .collect::<Vec<_>>(),
        vec!["classify", "fixture_algorithm"]
    );
    let classify = &control_flows[0];
    assert_eq!(classify.line, 1);
    assert_eq!(classify.line_span, 13);
    assert_eq!(classify.branch_count, 4);
    assert_eq!(classify.loop_count, 1);
    assert_eq!(classify.match_count, 1);
    assert_eq!(classify.max_nesting_depth, 3);
    assert_eq!(classify.max_loop_nesting_depth, 1);
    assert_eq!(classify.statement_count, 5);
    assert_eq!(classify.max_block_statement_count, 3);
    assert!(!classify.is_test_context);
    assert!(control_flows[1].is_test_context);
}
