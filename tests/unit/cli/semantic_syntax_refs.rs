#[path = "../../../src/parser/syntax_abi.rs"]
mod syntax_abi;

use syntax_abi::{RUST_OWNER_ITEMS_QUERY_REF, rust_syntax_abi_for_kind, syntax_atom_for_kind};

#[test]
fn rust_syntax_abi_is_the_single_mapping_for_common_item_kinds() {
    assert_eq!(
        RUST_OWNER_ITEMS_QUERY_REF,
        "semantic-tree-sitter-query/rust-owner-items.v1"
    );

    let function = rust_syntax_abi_for_kind("fn");
    assert_eq!(function.node_type, "function_item");
    assert_eq!(function.capture, "function.name");
    assert_eq!(function.field, Some("name"));
    assert_eq!(function.syn, "function_item/name");

    let import = rust_syntax_abi_for_kind("use");
    assert_eq!(import.node_type, "use_declaration");
    assert_eq!(import.capture, "import.name");
    assert_eq!(import.syn, "use_declaration/name");

    assert_eq!(syntax_atom_for_kind("impl"), "impl_item/name");
    assert_eq!(syntax_atom_for_kind("unknown"), "item/name");
}
