//! Parser-native verification task planning for external agent skills.

mod analysis;
mod api_path;
mod fingerprint;
mod model;
mod module_lookup;
mod performance;
mod planner;
mod profile;
mod profile_index;
mod render;
mod report;
mod report_entry;
mod report_manifest;
mod report_select;
mod report_write;
mod skill_descriptor;
mod task_builder;
mod task_index;

pub use analysis::{
    RustVerificationAnalysisProfile, RustVerificationPackageAnalysisProfile,
    build_rust_verification_analysis_profile, build_rust_verification_analysis_profile_with_config,
    render_rust_verification_analysis_profile, render_rust_verification_analysis_profile_json,
};
pub use model::{
    RustOwnerResponsibility, RustVerificationApiPathBaseline, RustVerificationDependencySignal,
    RustVerificationEvidence, RustVerificationPhase, RustVerificationPlan, RustVerificationPolicy,
    RustVerificationProfileHint, RustVerificationReceipt, RustVerificationReceiptStatus,
    RustVerificationReportObligation, RustVerificationRequirement, RustVerificationResolutionNote,
    RustVerificationSkillBinding, RustVerificationTask, RustVerificationTaskContract,
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
pub use report::{
    RustVerificationReportArtifact, RustVerificationReportArtifactRenderError,
    RustVerificationReportArtifactRole, RustVerificationReportBundle,
    RustVerificationReportOptions, RustVerificationReportPersistence,
    RustVerificationReportSidecar, RustVerificationReportSidecarRole,
    RustVerificationReportTemplate, RustVerificationReportTraceConfig,
    RustVerificationTraceMaxSeconds, RustVerificationTraceSampleIntervalMs,
    build_rust_verification_report_bundle, build_rust_verification_report_bundle_with_options,
    render_rust_verification_report_artifact_json,
    render_rust_verification_report_artifact_json_with_config,
    render_rust_verification_report_bundle, render_rust_verification_report_bundle_json,
};
pub use report_entry::{
    RustVerificationReportEntryAction, RustVerificationReportEntryAdvice,
    RustVerificationReportEntryArtifact, RustVerificationReportEntrySidecar,
    build_rust_verification_report_entry_advice,
    build_rust_verification_report_entry_advice_with_receipt,
    render_rust_verification_report_entry_advice,
    render_rust_verification_report_entry_advice_json,
};
pub use report_manifest::{
    RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID, RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_VERSION,
    RustVerificationReportManifestCompatibility, RustVerificationReportManifestSchema,
    check_rust_verification_report_manifest_schema,
    render_rust_verification_report_manifest_compatibility,
    render_rust_verification_report_manifest_schema_compatibility,
};
pub use report_select::{
    RustVerificationReportSelectionAdvice, RustVerificationReportSelectionArtifact,
    RustVerificationReportSelectionReason, RustVerificationReportSelectionScale,
    build_rust_verification_report_selection_advice,
    render_rust_verification_report_selection_advice,
    render_rust_verification_report_selection_advice_json,
};
pub use report_write::{
    RustVerificationReportArtifactWriteReceipt, RustVerificationReportSidecarWriteReceipt,
    RustVerificationReportWriteConfig, RustVerificationReportWriteError,
    RustVerificationReportWriteReceipt, render_rust_verification_report_write_receipt,
    render_rust_verification_report_write_receipt_json, write_rust_verification_reports,
    write_rust_verification_reports_with_options,
};
pub use skill_descriptor::RustVerificationSkillDescriptor;
pub use task_index::{
    RustVerificationTaskIndex, RustVerificationTaskRecord, build_rust_verification_task_index,
    render_rust_verification_task_index_json,
};
