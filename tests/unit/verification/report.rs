use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationProfileHint, RustVerificationReportOptions,
    RustVerificationReportPersistence, RustVerificationReportTraceConfig,
    RustVerificationSkillBinding, RustVerificationTaskKind, build_rust_verification_report_bundle,
    build_rust_verification_report_bundle_with_options, default_rust_harness_config,
    plan_rust_project_verification_with_config, render_rust_verification_plan,
    render_rust_verification_report_artifact_json, render_rust_verification_report_bundle_json,
};
use tempfile::TempDir;

use crate::verification::support::write_api_project;

#[test]
fn verification_report_bundle_materializes_required_artifacts() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_rust_harness_config()
        .with_verification_profile_hint(RustVerificationProfileHint::new(
            "src/api.rs",
            [RustOwnerResponsibility::LatencySensitive],
        ))
        .with_verification_skill_binding(
            RustVerificationTaskKind::Performance,
            RustVerificationSkillBinding::new("rust-verification-performance")
                .with_adapter("criterion"),
        );
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");

    let rendered = render_rust_verification_plan(&plan);
    let bundle = build_rust_verification_report_bundle(&plan);
    let json = render_rust_verification_report_bundle_json(&plan).expect("report json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");
    let perf_json = render_rust_verification_report_artifact_json(&plan, "performance_index_json")
        .expect("render performance artifact")
        .expect("performance artifact");
    let perf_value: serde_json::Value = serde_json::from_str(&perf_json).expect("parse perf json");

    assert!(rendered.contains("render_rust_verification_report_bundle_json"));
    assert_eq!(bundle.artifacts.len(), 2);
    assert_eq!(
        bundle
            .artifact("verification_plan_json")
            .expect("plan artifact")
            .task_count(),
        1
    );
    assert_eq!(
        bundle
            .artifact("performance_index_json")
            .expect("performance artifact")
            .persistence,
        RustVerificationReportPersistence::SourceBaseline
    );
    assert_eq!(
        bundle
            .artifact("verification_plan_json")
            .expect("plan artifact")
            .persistence,
        RustVerificationReportPersistence::RuntimeCache
    );
    assert_eq!(bundle.source_baseline_artifacts().len(), 1);
    assert_eq!(bundle.runtime_cache_artifacts().len(), 1);
    assert_eq!(
        bundle
            .artifact("performance_index_json")
            .expect("performance artifact")
            .trace
            .as_ref()
            .expect("trace")
            .max_seconds,
        Some(300)
    );
    assert_eq!(perf_value["records"][0]["state"], "pending");
    assert_eq!(
        value["artifacts"][0]["artifact_name"],
        "verification_plan.json"
    );
    assert_eq!(
        value["artifacts"][1]["artifact_name"],
        "performance_index.json"
    );
    assert_eq!(
        value["artifacts"][1]["template"]["template_id"],
        "performance-index"
    );
    assert!(value["artifacts"][1].get("payload").is_none());
}

#[test]
fn verification_report_bundle_allows_agent_trace_overrides() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_rust_harness_config()
        .with_verification_profile_hint(RustVerificationProfileHint::new(
            "src/api.rs",
            [RustOwnerResponsibility::LatencySensitive],
        ))
        .with_verification_skill_binding(
            RustVerificationTaskKind::Performance,
            RustVerificationSkillBinding::new("rust-verification-performance")
                .with_adapter("criterion"),
        );
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let options = RustVerificationReportOptions::default().with_artifact_trace(
        "performance_index_json",
        RustVerificationReportTraceConfig::new("agent-fast-performance")
            .with_max_seconds(45)
            .with_sample_interval_ms(500),
    );

    let bundle = build_rust_verification_report_bundle_with_options(&plan, &options);
    let performance = bundle
        .artifact("performance_index_json")
        .expect("performance artifact");

    assert_eq!(
        performance.trace.as_ref().expect("trace").profile,
        "agent-fast-performance"
    );
    assert_eq!(
        performance.trace.as_ref().expect("trace").max_seconds,
        Some(45)
    );
}

#[test]
fn verification_report_bundle_is_empty_without_active_tasks() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let plan = plan_rust_project_verification_with_config(root, &default_rust_harness_config())
        .expect("plan");

    let bundle = build_rust_verification_report_bundle(&plan);
    let json = render_rust_verification_report_bundle_json(&plan).expect("report json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");

    assert!(bundle.is_empty());
    assert_eq!(value["artifacts"], serde_json::json!([]));
}
