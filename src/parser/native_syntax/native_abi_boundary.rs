//! Native Rust ABI boundary facts.

use quote::ToTokens;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustNativeAbiSurfaceSyntax {
    pub line: usize,
    pub item_kind: &'static str,
    pub item_name: String,
    pub has_abi_version_const: bool,
    pub has_abi_id_const: bool,
    pub has_header_path_const: bool,
    pub has_header_source_const: bool,
    pub has_extern_block: bool,
    pub has_native_abi_marker: bool,
    pub is_test_context: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeAbiModuleContract {
    has_abi_version_const: bool,
    has_abi_id_const: bool,
    has_header_path_const: bool,
    has_header_source_const: bool,
    has_extern_block: bool,
}

pub(crate) fn native_abi_surface_syntax(
    items: &[syn::Item],
    source_file: &std::path::Path,
) -> Vec<RustNativeAbiSurfaceSyntax> {
    let module_contract = native_abi_module_contract(items);
    let source_path = source_file.to_string_lossy().replace('\\', "/");

    items
        .iter()
        .filter_map(|item| {
            let (item_kind, ident, attrs, vis) = match item {
                syn::Item::Struct(item_struct) => (
                    "struct",
                    &item_struct.ident,
                    &item_struct.attrs,
                    &item_struct.vis,
                ),
                syn::Item::Enum(item_enum) => {
                    ("enum", &item_enum.ident, &item_enum.attrs, &item_enum.vis)
                }
                syn::Item::Union(item_union) => (
                    "union",
                    &item_union.ident,
                    &item_union.attrs,
                    &item_union.vis,
                ),
                _ => return None,
            };
            if !is_public_visibility(vis) || !attrs_have_repr_c(attrs) {
                return None;
            }
            let item_name = ident.to_string();
            let has_native_abi_marker = module_contract.has_extern_block
                || module_contract.has_abi_version_const
                || module_contract.has_abi_id_const
                || module_contract.has_header_path_const
                || module_contract.has_header_source_const
                || contains_native_abi_marker(&source_path)
                || contains_native_abi_marker(&item_name);
            Some(RustNativeAbiSurfaceSyntax {
                line: ident.span().start().line.max(1),
                item_kind,
                item_name,
                has_abi_version_const: module_contract.has_abi_version_const,
                has_abi_id_const: module_contract.has_abi_id_const,
                has_header_path_const: module_contract.has_header_path_const,
                has_header_source_const: module_contract.has_header_source_const,
                has_extern_block: module_contract.has_extern_block,
                has_native_abi_marker,
                is_test_context: attrs_have_cfg_test(attrs),
            })
        })
        .collect()
}

fn native_abi_module_contract(items: &[syn::Item]) -> NativeAbiModuleContract {
    let mut contract = NativeAbiModuleContract {
        has_abi_version_const: false,
        has_abi_id_const: false,
        has_header_path_const: false,
        has_header_source_const: false,
        has_extern_block: false,
    };
    for item in items {
        match item {
            syn::Item::Const(item_const) => {
                let name = item_const.ident.to_string().to_ascii_uppercase();
                contract.has_abi_version_const |= name.contains("ABI_VERSION");
                contract.has_abi_id_const |= name.contains("ABI_ID");
                contract.has_header_path_const |= name.contains("HEADER_PATH");
                contract.has_header_source_const |= name.contains("HEADER_SOURCE");
            }
            syn::Item::ForeignMod(_) => contract.has_extern_block = true,
            _ => {}
        }
    }
    contract
}

fn attrs_have_repr_c(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("repr")
            && attr
                .meta
                .to_token_stream()
                .to_string()
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .any(|token| token == "C")
    })
}

fn attrs_have_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(attribute_has_cfg_test)
}

fn attribute_has_cfg_test(attr: &syn::Attribute) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    let syn::Meta::List(list) = &attr.meta else {
        return false;
    };
    list.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
        .map(|metas| {
            metas
                .iter()
                .any(|meta| matches!(meta, syn::Meta::Path(path) if path.is_ident("test")))
        })
        .unwrap_or(false)
}

fn contains_native_abi_marker(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    normalized.contains("native")
        || normalized.contains("abi")
        || normalized.contains("ffi")
        || normalized.contains("extern")
        || normalized.contains("bindings")
}

fn is_public_visibility(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}
