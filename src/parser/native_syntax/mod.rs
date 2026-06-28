//! Native Rust syntax facts shared by harness policies.

mod async_queue_boundary;
mod collect;
mod control_flow;
mod data_shape;
mod facts;
mod invocation_facts;
mod item_facts;
pub(crate) mod item_projection;
mod module_facts;
mod path_facts;
#[cfg(any(feature = "search", feature = "cli"))]
pub(crate) mod projection_code;
mod select_cancellation_safety;
pub(crate) mod signature;
mod sync_lock_boundary;
mod timeout_cancellation_safety;

pub(crate) use collect::rust_native_syntax_facts;
pub(crate) use control_flow::RustFunctionControlFlowSyntax;
pub(crate) use data_shape::{
    RustPublicEnumTupleVariantFieldSyntax, RustPublicEnumVariantFieldSyntax,
    RustPublicStructFieldSyntax, RustPublicTypeAliasSyntax,
};
pub(crate) use facts::{RustNativeSyntaxFacts, RustTopLevelItemSyntax};
