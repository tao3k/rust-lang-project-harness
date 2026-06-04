//! Native Rust invocation facts.

use proc_macro2::{TokenStream, TokenTree};
use syn::spanned::Spanned;

use super::facts::RustInvocationSyntax;

pub(super) fn invocation_syntax(path: &syn::Path) -> Option<RustInvocationSyntax> {
    let terminal_name = path.segments.last()?.ident.to_string();
    Some(RustInvocationSyntax {
        line: path.span().start().line.max(1),
        terminal_name,
        argument_token_count: 0,
        argument_top_level_idents: Vec::new(),
    })
}

pub(super) fn macro_invocation_syntax(mac: &syn::Macro) -> Option<RustInvocationSyntax> {
    let mut invocation = invocation_syntax(&mac.path)?;
    invocation.argument_token_count = mac.tokens.clone().into_iter().count();
    invocation.argument_top_level_idents = token_stream_top_level_idents(mac.tokens.clone());
    Some(invocation)
}

pub(super) fn function_call_invocation_syntax(
    path: &syn::Path,
    arg_count: usize,
) -> Option<RustInvocationSyntax> {
    let mut invocation = invocation_syntax(path)?;
    invocation.argument_token_count = arg_count;
    Some(invocation)
}

pub(super) fn include_literal_target(mac: &syn::Macro) -> Option<String> {
    let invocation = invocation_syntax(&mac.path)?;
    if invocation.terminal_name != "include" {
        return None;
    }
    syn::parse2::<syn::LitStr>(mac.tokens.clone())
        .ok()
        .map(|lit| lit.value())
}

fn token_stream_top_level_idents(tokens: TokenStream) -> Vec<String> {
    tokens
        .into_iter()
        .filter_map(|token| match token {
            TokenTree::Ident(ident) => Some(ident.to_string()),
            TokenTree::Group(_) | TokenTree::Punct(_) | TokenTree::Literal(_) => None,
        })
        .collect()
}
