//! Native Rust syntax facts shared by harness policies.

mod collect;
mod control_flow;
mod data_shape;
mod signature;

pub(crate) use collect::{RustNativeSyntaxFacts, RustTopLevelItemSyntax, rust_native_syntax_facts};
pub(crate) use control_flow::RustFunctionControlFlowSyntax;
pub(crate) use data_shape::{
    RustPublicEnumTupleVariantFieldSyntax, RustPublicEnumVariantFieldSyntax,
    RustPublicStructFieldSyntax,
};
