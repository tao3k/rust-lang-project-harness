//! Parser-native verification task planning for external agent skills.

mod fingerprint;
mod model;
mod planner;
mod profile;
mod render;

pub use model::{
    RustOwnerResponsibility, RustVerificationEvidence, RustVerificationPhase, RustVerificationPlan,
    RustVerificationPolicy, RustVerificationProfileHint, RustVerificationReceipt,
    RustVerificationReceiptStatus, RustVerificationRequirement, RustVerificationResolutionNote,
    RustVerificationTask, RustVerificationTaskContract, RustVerificationTaskKind,
    RustVerificationTaskState, RustVerificationWaiver,
};
pub use planner::{
    plan_rust_project_verification, plan_rust_project_verification_with_config,
    plan_rust_project_verification_with_policy,
};
pub use render::{render_rust_verification_plan, render_rust_verification_plan_json};
