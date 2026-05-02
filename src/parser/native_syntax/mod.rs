//! Native Rust syntax facts shared by harness policies.

mod collect;
mod signature;

pub(crate) use collect::{RustNativeSyntaxFacts, RustTopLevelItemSyntax, rust_native_syntax_facts};
