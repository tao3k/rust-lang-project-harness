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
