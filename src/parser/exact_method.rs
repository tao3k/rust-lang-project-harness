use syn::spanned::Spanned;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RustExactMethod {
    pub(crate) impl_owner: String,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
}

pub(crate) fn parse_rust_exact_methods(
    source: &str,
    method_name: &str,
) -> Result<Vec<RustExactMethod>, String> {
    let syntax = super::parse_rust_source_syntax(source).map_err(|error| error.to_string())?;
    let mut methods = Vec::new();
    for item in &syntax.items {
        let syn::Item::Impl(item_impl) = item else {
            continue;
        };
        let impl_owner = exact_impl_type_name(item_impl.self_ty.as_ref())
            .unwrap_or_else(|| "<unknown>".to_string());
        for impl_item in &item_impl.items {
            let syn::ImplItem::Fn(method) = impl_item else {
                continue;
            };
            if method.sig.ident != method_name {
                continue;
            }
            let method_span = method.span();
            let start_line = method
                .attrs
                .first()
                .map(Spanned::span)
                .map(|span| span.start().line)
                .unwrap_or_else(|| method_span.start().line);
            methods.push(RustExactMethod {
                impl_owner: impl_owner.clone(),
                start_line,
                end_line: method_span.end().line,
            });
        }
    }
    Ok(methods)
}

fn exact_impl_type_name(item_type: &syn::Type) -> Option<String> {
    let syn::Type::Path(type_path) = item_type else {
        return None;
    };
    type_path
        .path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}
