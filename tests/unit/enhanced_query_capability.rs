use std::collections::BTreeSet;
use std::path::Path;

const CORPUS: &str = r#"
pub fn run() {}
pub struct Data;
pub enum Choice { A }
pub trait Work {}
impl Work for Data {}
mod inner {}
use std::fmt;
const LIMIT: u8 = 1;
static FLAG: bool = true;
type Alias = u8;
macro_rules! sample { () => {} }
"#;

const ADMITTED_NODE_TYPES: &[&str] = &[
    "const_item",
    "enum_item",
    "function_item",
    "impl_item",
    "mod_item",
    "static_item",
    "struct_item",
    "trait_item",
    "type_item",
    "use_declaration",
];

#[test]
fn native_parser_and_tree_sitter_have_the_same_admitted_top_level_match_set() {
    let native = crate::parser::parse_rust_source(Path::new("src/lib.rs"), CORPUS.to_owned());
    assert!(native.report.is_valid);
    let native_rows = native
        .syntax_facts
        .top_level_items
        .iter()
        .filter_map(|item| {
            native_kind_to_tree_sitter(item.kind)
                .map(|kind| (kind.to_owned(), item.name.clone().unwrap_or_default()))
        })
        .collect::<BTreeSet<_>>();

    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_rust::LANGUAGE.into();
    parser
        .set_language(&language)
        .expect("tree-sitter Rust language must load");
    let tree = parser.parse(CORPUS, None).expect("tree-sitter parse tree");
    assert!(!tree.root_node().has_error());
    let mut cursor = tree.root_node().walk();
    let tree_sitter_rows = tree
        .root_node()
        .named_children(&mut cursor)
        .filter(|node| ADMITTED_NODE_TYPES.contains(&node.kind()))
        .map(|node| {
            let name = node
                .child_by_field_name("name")
                .and_then(|name| name.utf8_text(CORPUS.as_bytes()).ok())
                .unwrap_or_default()
                .to_owned();
            (node.kind().to_owned(), name)
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(native_rows, tree_sitter_rows);

    let parser_abi_digest = framed_digest(
        "asp-rust-syn-native-items-v1",
        &[
            include_bytes!("../../src/parser/parsed_module.rs"),
            include_bytes!("../../src/parser/native_syntax/collect.rs"),
            include_bytes!("../../src/parser/native_syntax/item_facts.rs"),
            include_bytes!("../../src/parser/native_syntax/facts.rs"),
        ],
    );
    let query_grammar_digest = framed_digest(
        "asp-rust-tree-sitter-query-grammar-v1",
        &[include_bytes!(
            "../../tree-sitter/tree-sitter-rust/grammar-profile.json"
        )],
    );
    let corpus_digest = framed_digest("asp-rust-enhanced-query-corpus-v1", &[CORPUS.as_bytes()]);
    let mut receipt_hasher = blake3::Hasher::new();
    digest_field(
        &mut receipt_hasher,
        b"asp-rust-enhanced-query-equivalence-receipt-v1",
    );
    digest_field(&mut receipt_hasher, CORPUS.as_bytes());
    for (kind, name) in &native_rows {
        digest_field(&mut receipt_hasher, kind.as_bytes());
        digest_field(&mut receipt_hasher, name.as_bytes());
    }
    let receipt_digest = format!("blake3-256:{}", receipt_hasher.finalize().to_hex());
    assert_eq!(
        parser_abi_digest,
        "blake3-256:122dae5e98853c9a8a168a7ee9b558d64a200bf4c97516f51f7171293883a4ca"
    );
    assert_eq!(
        query_grammar_digest,
        "blake3-256:8cce2ee31e888123bd915c9cfec46c957df23767033c4b1162cde54fcb1ab14f"
    );
    assert_eq!(
        corpus_digest,
        "blake3-256:d44f6ab7b8426640d3e400910c9745d5fdfc3c20f5bb3219560b5a5bc5de1973"
    );
    assert_eq!(
        receipt_digest,
        "blake3-256:2cd7fd4a2451386e2d6c2522cddded95ab475ab836ec15992d3e13f60a4e0bac"
    );
}

#[test]
fn capability_table_is_bound_to_the_executable_differential_receipt() {
    let mut table: serde_json::Value = serde_json::from_str(include_str!(
        "../../tree-sitter/tree-sitter-rust/enhanced-query-capabilities.v1.json"
    ))
    .expect("enhanced-query capability table JSON");
    assert_eq!(table["schemaVersion"], "1");
    assert_eq!(table["languageId"], "rust");
    assert_eq!(table["providerId"], "asp-rust");
    assert_eq!(
        table["operatorTableDigest"],
        "blake3-256:e9f88db6f6cab915cad26739fd9c9da58e1dbd8e73b7e314adb08ff6fca45d2a"
    );
    assert_eq!(
        table["tableDigest"],
        "blake3-256:55846cb3ab907973f4797032e44a28091bb29ef169f973ee463382461325036a"
    );

    let rows = table["rows"].as_array().expect("capability rows");
    assert!(rows.iter().any(|row| {
        row["rowId"] == "rust.node.macro-definition"
            && row["publicationState"] == "provider-local"
            && row.get("equivalenceEvidence").is_none()
    }));
    for row in rows
        .iter()
        .filter(|row| row["publicationState"] == "runtime")
    {
        let evidence = &row["equivalenceEvidence"];
        assert_eq!(
            evidence["corpusDigest"],
            "blake3-256:d44f6ab7b8426640d3e400910c9745d5fdfc3c20f5bb3219560b5a5bc5de1973"
        );
        assert_eq!(
            evidence["receiptDigest"],
            "blake3-256:2cd7fd4a2451386e2d6c2522cddded95ab475ab836ec15992d3e13f60a4e0bac"
        );
    }

    let expected_table_digest = table["tableDigest"]
        .as_str()
        .expect("tableDigest")
        .to_owned();
    table
        .as_object_mut()
        .expect("capability table object")
        .remove("tableDigest");
    let canonical = serde_json::to_vec(&table).expect("canonical capability table JSON");
    let actual_table_digest = format!("blake3-256:{}", blake3::hash(&canonical).to_hex());
    assert_eq!(expected_table_digest, actual_table_digest);
}

fn framed_digest(domain: &str, fields: &[&[u8]]) -> String {
    let mut hasher = blake3::Hasher::new();
    digest_field(&mut hasher, domain.as_bytes());
    for field in fields {
        digest_field(&mut hasher, field);
    }
    format!("blake3-256:{}", hasher.finalize().to_hex())
}

fn digest_field(hasher: &mut blake3::Hasher, value: &[u8]) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value);
}

fn native_kind_to_tree_sitter(kind: &str) -> Option<&'static str> {
    match kind {
        "fn" | "function" => Some("function_item"),
        "struct" => Some("struct_item"),
        "enum" => Some("enum_item"),
        "trait" => Some("trait_item"),
        "impl" => Some("impl_item"),
        "mod" => Some("mod_item"),
        "use" => Some("use_declaration"),
        "const" => Some("const_item"),
        "static" => Some("static_item"),
        "type" => Some("type_item"),
        _ => None,
    }
}
