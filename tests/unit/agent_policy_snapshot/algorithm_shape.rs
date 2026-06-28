use std::fs;

use tempfile::TempDir;

use super::{assert_agent_snapshot, write_manifest};

#[test]
fn agent_r015_nested_algorithm_shape_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r015-algorithm-shape");
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

    assert_agent_snapshot(
        root,
        "RUST-AGENT-CFG-PUBLIC-015",
        1,
        "agent_r015_nested_algorithm_shape",
    );
}

#[test]
fn agent_r015_literal_dispatch_chain_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r015-literal-dispatch");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(root.join("src/api.rs"), literal_dispatch_source()).expect("write api");

    assert_agent_snapshot(
        root,
        "RUST-AGENT-CFG-PUBLIC-015",
        1,
        "agent_r015_literal_dispatch_chain",
    );
}

#[test]
fn agent_r016_broad_linear_algorithm_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r016-broad-linear");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(root.join("src/api.rs"), broad_linear_algorithm_source()).expect("write api");

    assert_agent_snapshot(
        root,
        "RUST-AGENT-CFG-PUBLIC-016",
        1,
        "agent_r016_broad_linear_algorithm",
    );
}

#[test]
fn agent_r017_native_iterator_idiom_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r017-native-iterator");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(root.join("src/api.rs"), manual_iterator_source()).expect("write api");

    assert_agent_snapshot(
        root,
        "RUST-AGENT-ITER-PUBLIC-017",
        1,
        "agent_r017_native_iterator_idiom",
    );
}

#[test]
fn agent_r025_internal_traversal_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r025-internal-traversal");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod receipt;\n").expect("write lib");
    fs::write(root.join("src/receipt.rs"), internal_traversal_source()).expect("write receipt");

    assert_agent_snapshot(
        root,
        "RUST-AGENT-CFG-IMPL-025",
        1,
        "agent_r025_internal_traversal",
    );
}

#[test]
fn agent_r026_internal_iterator_idiom_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r026-internal-iterator");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod receipt;\n").expect("write lib");
    fs::write(root.join("src/receipt.rs"), internal_iterator_source()).expect("write receipt");

    assert_agent_snapshot(
        root,
        "RUST-AGENT-ITER-IMPL-026",
        1,
        "agent_r026_internal_iterator_idiom",
    );
}

fn broad_linear_algorithm_source() -> String {
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

fn internal_traversal_source() -> String {
    "//! Receipt helpers.\n\
     struct ContractReceipt {\n\
     \tsummary: ReceiptSummary,\n\
     \trepositories: Vec<RepositoryReceipt>,\n\
     }\n\
     struct ReceiptSummary { failed_query_count: usize }\n\
     struct RepositoryReceipt { query_receipts: Vec<QueryReceipt> }\n\
     struct QueryReceipt { passed: bool, name: String }\n\
     fn collect_failed_queries(receipt: &ContractReceipt) -> Vec<String> {\n\
     \tlet mut failed = Vec::new();\n\
     \tif receipt.summary.failed_query_count > 0 {\n\
     \t\tfor repository in &receipt.repositories {\n\
     \t\t\tfor query in &repository.query_receipts {\n\
     \t\t\t\tif !query.passed {\n\
     \t\t\t\t\tfailed.push(query.name.clone());\n\
     \t\t\t\t}\n\
     \t\t\t}\n\
     \t\t}\n\
     \t}\n\
     \tfailed\n\
     }\n"
    .to_string()
}

fn internal_iterator_source() -> String {
    "//! Receipt helpers.\n\
     struct QueryReceipt { passed: bool, name: String }\n\
     fn failed_query_names(queries: &[QueryReceipt]) -> Vec<String> {\n\
     \tlet mut names = Vec::new();\n\
     \tfor query in queries {\n\
     \t\tif !query.passed {\n\
     \t\t\tnames.push(query.name.clone());\n\
     \t\t}\n\
     \t}\n\
     \tnames\n\
     }\n"
    .to_string()
}
