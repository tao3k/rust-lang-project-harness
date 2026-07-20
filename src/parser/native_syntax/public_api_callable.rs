use super::data_shape::is_public_visibility;
use super::facts::RustPublicApiCallableSyntax;
use super::module_facts::{attrs_have_cfg_test, attrs_have_doc};

pub(super) fn public_api_callable_syntax(item: &syn::Item) -> Vec<RustPublicApiCallableSyntax> {
    match item {
        syn::Item::Fn(item_fn) if is_public_visibility(&item_fn.vis) => {
            vec![RustPublicApiCallableSyntax {
                line: item_fn.sig.ident.span().start().line.max(1),
                kind: "fn",
                name: item_fn.sig.ident.to_string(),
                has_doc: attrs_have_doc(&item_fn.attrs),
                is_public: true,
                is_test_context: attrs_have_cfg_test(&item_fn.attrs),
            }]
        }
        syn::Item::Impl(item_impl) => item_impl
            .items
            .iter()
            .filter_map(|item| match item {
                syn::ImplItem::Fn(method) if impl_method_is_public_api(item_impl, method) => {
                    Some(RustPublicApiCallableSyntax {
                        line: method.sig.ident.span().start().line.max(1),
                        kind: "method",
                        name: method.sig.ident.to_string(),
                        has_doc: attrs_have_doc(&method.attrs),
                        is_public: true,
                        is_test_context: attrs_have_cfg_test(&item_impl.attrs)
                            || attrs_have_cfg_test(&method.attrs),
                    })
                }
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn impl_method_is_public_api(item_impl: &syn::ItemImpl, method: &syn::ImplItemFn) -> bool {
    item_impl.trait_.is_some() || is_public_visibility(&method.vis)
}
