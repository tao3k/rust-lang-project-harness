use std::fs;

use rust_lang_project_harness::run_rust_project_harness;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn public_nested_algorithm_shape_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "nested-algorithm-shape");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Classifies rows.\n\
         pub fn classify(rows: &[usize], enabled: bool) -> usize {\n\
         \tlet mut total = 0;\n\
         \tfor row in rows {\n\
         \t\tif enabled {\n\
         \t\t\tif *row > 10 {\n\
         \t\t\t\tif *row < 20 {\n\
         \t\t\t\t\ttotal += *row;\n\
         \t\t\t\t}\n\
         \t\t\t}\n\
         \t\t}\n\
         \t}\n\
         \ttotal\n\
         }\n",
    )
    .expect("write api");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "AGENT-R015");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("deep control-flow nesting"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_broad_linear_algorithm_surface_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "broad-linear-algorithm");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(root.join("src/api.rs"), broad_linear_source()).expect("write api");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "AGENT-R016");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("large linear statement block"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_match_dispatch_is_not_nested_algorithm_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "match-dispatch-algorithm");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Classifies a route.\n\
         pub fn classify(kind: &str) -> usize {\n\
         \tmatch kind {\n\
         \t\t\"alpha\" => 1,\n\
         \t\t\"beta\" => 2,\n\
         \t\t\"gamma\" => 3,\n\
         \t\t\"delta\" => 4,\n\
         \t\t\"epsilon\" => 5,\n\
         \t\t\"zeta\" => 6,\n\
         \t\t\"eta\" => 7,\n\
         \t\t_ => 0,\n\
         \t}\n\
         }\n",
    )
    .expect("write api");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(findings_for_rule(&report, "AGENT-R015").is_empty());
    assert!(findings_for_rule(&report, "AGENT-R016").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_literal_dispatch_chain_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "literal-dispatch-chain");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(root.join("src/api.rs"), literal_dispatch_source()).expect("write api");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "AGENT-R015");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("literal dispatch chain without match")
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_manual_iterator_boilerplate_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "manual-iterator-boilerplate");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(root.join("src/api.rs"), manual_iterator_source()).expect("write api");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "AGENT-R017");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("manual collection accumulator loop")
    );
    assert!(findings[0].summary.contains("manual predicate loop"));
    assert!(findings[0].summary.contains("manual count loop"));
    assert!(
        findings[0]
            .summary
            .contains("manual numeric accumulator loop")
    );
    assert!(
        findings[0]
            .summary
            .contains("repeated pass over the same iterator source")
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn deeply_nested_algorithm_does_not_duplicate_native_iterator_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "nested-algorithm-no-native-duplicate");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Classifies rows.\n\
         pub fn classify(rows: &[usize], enabled: bool) -> usize {\n\
         \tlet mut total = 0;\n\
         \tfor row in rows {\n\
         \t\tif enabled {\n\
         \t\t\tif *row > 10 {\n\
         \t\t\t\tif *row < 20 {\n\
         \t\t\t\t\ttotal += 1;\n\
         \t\t\t\t}\n\
         \t\t\t}\n\
         \t\t}\n\
         \t}\n\
         \ttotal\n\
         }\n",
    )
    .expect("write api");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert_eq!(findings_for_rule(&report, "AGENT-R015").len(), 1);
    assert!(findings_for_rule(&report, "AGENT-R017").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

fn broad_linear_source() -> String {
    let mut source = String::from(
        "//! Public API owner.\n\
         /// Summarizes values.\n\
         pub fn summarize(value: usize) -> usize {\n",
    );
    for index in 0..15 {
        source.push_str(&format!("    let step_{index} = value + {index};\n"));
    }
    source.push_str("    step_0\n}\n");
    source
}

fn manual_iterator_source() -> String {
    "//! Public API owner.\n\
     /// Summarizes values.\n\
     pub fn summarize(values: &[usize]) -> bool {\n\
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
     }\n"
    .to_string()
}

fn literal_dispatch_source() -> String {
    "//! Public API owner.\n\
     /// Routes a kind.\n\
     pub fn route(kind: &str) -> usize {\n\
     \tif kind == \"alpha\" {\n\
     \t\t1\n\
     \t} else if kind == \"beta\" {\n\
     \t\t2\n\
     \t} else if kind == \"gamma\" {\n\
     \t\t3\n\
     \t} else {\n\
     \t\t0\n\
     \t}\n\
     }\n"
    .to_string()
}
