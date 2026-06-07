//! Rust tree-sitter-compatible syntax ABI atoms.

pub(crate) const RUST_OWNER_ITEMS_QUERY_REF: &str =
    "semantic-tree-sitter-query/rust-owner-items.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RustSyntaxAbi {
    pub(crate) node_type: &'static str,
    pub(crate) capture: &'static str,
    pub(crate) field: Option<&'static str>,
    pub(crate) syn: &'static str,
}

impl RustSyntaxAbi {
    fn new(node_type: &'static str, capture: &'static str, syn: &'static str) -> Self {
        Self {
            node_type,
            capture,
            field: capture_field(capture),
            syn,
        }
    }
}

pub(crate) fn rust_syntax_abi_for_kind(kind: &str) -> RustSyntaxAbi {
    match kind {
        "const" => RustSyntaxAbi::new("const_item", "constant.name", "const_item/name"),
        "enum" => RustSyntaxAbi::new("enum_item", "enum.name", "enum_item/name"),
        "extern_crate" => RustSyntaxAbi::new(
            "extern_crate_declaration",
            "extern.name",
            "extern_crate_declaration/name",
        ),
        "fn" => RustSyntaxAbi::new("function_item", "function.name", "function_item/name"),
        "impl" => RustSyntaxAbi::new("impl_item", "impl.name", "impl_item/name"),
        "macro" => RustSyntaxAbi::new("macro_definition", "macro.name", "macro_definition/name"),
        "mod" => RustSyntaxAbi::new("mod_item", "module.name", "mod_item/name"),
        "static" => RustSyntaxAbi::new("static_item", "constant.name", "static_item/name"),
        "struct" => RustSyntaxAbi::new("struct_item", "struct.name", "struct_item/name"),
        "trait" | "trait_alias" => {
            RustSyntaxAbi::new("trait_item", "trait.name", "trait_item/name")
        }
        "type" => RustSyntaxAbi::new("type_item", "type.name", "type_item/name"),
        "use" => RustSyntaxAbi::new("use_declaration", "import.name", "use_declaration/name"),
        _ => RustSyntaxAbi::new("item", "item.name", "item/name"),
    }
}

pub(crate) fn syntax_atom_for_kind(kind: &str) -> &'static str {
    rust_syntax_abi_for_kind(kind).syn
}

fn capture_field(capture: &str) -> Option<&'static str> {
    if capture.ends_with(".name") {
        Some("name")
    } else {
        None
    }
}
