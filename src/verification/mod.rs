//! Parser-native verification task planning for external agent skills.

mod fingerprint;
mod model;
mod performance;
mod planner;
mod profile;
mod profile_index;
mod render;

pub use model::{
    RustOwnerResponsibility, RustVerificationDependencySignal, RustVerificationEvidence,
    RustVerificationPhase, RustVerificationPlan, RustVerificationPolicy,
    RustVerificationProfileHint, RustVerificationReceipt, RustVerificationReceiptStatus,
    RustVerificationRequirement, RustVerificationResolutionNote, RustVerificationSkillBinding,
    RustVerificationSkillDescriptor, RustVerificationTask, RustVerificationTaskContract,
    RustVerificationTaskKind, RustVerificationTaskState, RustVerificationWaiver,
};
pub use performance::{
    RustVerificationPerformanceIndex, RustVerificationPerformanceRecord,
    build_rust_verification_performance_index, render_rust_verification_performance_index,
    render_rust_verification_performance_index_json,
};
pub use planner::{
    plan_rust_project_verification, plan_rust_project_verification_with_config,
    plan_rust_project_verification_with_policy,
};
pub use profile_index::{
    RustVerificationProfileCandidate, RustVerificationProfileCandidateState,
    RustVerificationProfileIndex, build_rust_verification_profile_index,
    build_rust_verification_profile_index_with_config,
    build_rust_verification_profile_index_with_policy, render_rust_verification_profile_index,
    render_rust_verification_profile_index_json,
};
pub use render::{
    render_rust_verification_plan, render_rust_verification_plan_json,
    render_rust_verification_skill_contracts,
};
