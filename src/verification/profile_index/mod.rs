//! Parser-native responsibility profile suggestions for verification config.

mod collect;
mod model;
mod render;
mod scan;
mod taxonomy;

pub use model::{
    RustVerificationProfileCandidate, RustVerificationProfileCandidateState,
    RustVerificationProfileIndex,
};
pub use render::{
    render_rust_verification_profile_index, render_rust_verification_profile_index_json,
};
pub use scan::{
    build_rust_verification_profile_index, build_rust_verification_profile_index_with_config,
    build_rust_verification_profile_index_with_policy,
};
