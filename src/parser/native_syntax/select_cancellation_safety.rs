//! Tokio select cancellation-safety syntax facts.

use proc_macro2::{TokenStream, TokenTree};
use syn::visit::{self, Visit};

pub(crate) fn tokio_select_cancel_unsafe_io_count(block: &syn::Block) -> usize {
    let mut collector = SelectCancellationSafetyCollector::default();
    collector.visit_block(block);
    collector.cancel_unsafe_io_calls
}

#[derive(Default)]
struct SelectCancellationSafetyCollector {
    cancel_unsafe_io_calls: usize,
}

impl<'ast> Visit<'ast> for SelectCancellationSafetyCollector {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if macro_path_is_tokio_select(&mac.path) {
            self.cancel_unsafe_io_calls += cancel_unsafe_io_ident_count(&mac.tokens);
        }
        visit::visit_macro(self, mac);
    }
}

fn macro_path_is_tokio_select(path: &syn::Path) -> bool {
    let mut segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string());
    let Some(first) = segments.next() else {
        return false;
    };
    let second = segments.next();
    match second {
        Some(second) => first == "tokio" && second == "select" && segments.next().is_none(),
        None => first == "select",
    }
}

fn cancel_unsafe_io_ident_count(tokens: &TokenStream) -> usize {
    tokens.clone().into_iter().map(token_tree_count).sum()
}

fn token_tree_count(token: TokenTree) -> usize {
    match token {
        TokenTree::Ident(ident) if is_cancel_unsafe_io_name(&ident.to_string()) => 1,
        TokenTree::Group(group) => cancel_unsafe_io_ident_count(&group.stream()),
        _ => 0,
    }
}

pub(crate) fn is_cancel_unsafe_io_name(name: &str) -> bool {
    matches!(
        name,
        "read_exact" | "read_to_end" | "read_to_string" | "write_all" | "write_all_buf"
    )
}
