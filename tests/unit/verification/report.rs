use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationProfileHint, RustVerificationReportOptions,
    RustVerificationReportPersistence, RustVerificationReportTraceConfig,
    RustVerificationReportWriteConfig, RustVerificationSkillBinding, RustVerificationTaskKind,
    RustVerificationTraceMaxSeconds, build_rust_verification_report_bundle,
    build_rust_verification_report_bundle_with_options, default_rust_harness_config,
    plan_rust_project_verification_with_config, render_rust_verification_plan,
    render_rust_verification_report_artifact_json, render_rust_verification_report_bundle_json,
    write_rust_verification_reports,
};
use std::path::PathBuf;
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
    let task_json = render_rust_verification_report_artifact_json(&plan, "task_index_json")
        .expect("render task artifact")
        .expect("task artifact");
    let task_value: serde_json::Value = serde_json::from_str(&task_json).expect("parse task json");

    assert!(rendered.contains("render_rust_verification_report_bundle_json"));
    assert_eq!(bundle.artifacts.len(), 3);
    assert_eq!(
        bundle
            .artifact("verification_plan_json")
            .expect("plan artifact")
            .task_count(),
        1
    );
    assert_eq!(
        bundle
            .artifact("task_index_json")
            .expect("task artifact")
            .persistence,
        RustVerificationReportPersistence::SourceBaseline
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
    assert_eq!(bundle.source_baseline_artifacts().len(), 2);
    assert_eq!(bundle.runtime_cache_artifacts().len(), 1);
    assert_eq!(
        bundle
            .artifact("performance_index_json")
            .expect("performance artifact")
            .trace
            .as_ref()
            .expect("trace")
            .max_seconds,
        Some(RustVerificationTraceMaxSeconds::new(300))
    );
    assert_eq!(perf_value["records"][0]["state"], "pending");
    assert_eq!(task_value["records"][0]["kind"], "performance");
    assert_eq!(
        value["artifacts"][0]["artifact_name"],
        "verification_plan.json"
    );
    assert_eq!(value["artifacts"][1]["artifact_name"], "task_index.json");
    assert_eq!(
        value["artifacts"][2]["artifact_name"],
        "performance_index.json"
    );
    assert_eq!(
        value["artifacts"][2]["template"]["template_id"],
        "performance-index"
    );
    assert!(value["artifacts"][2].get("payload").is_none());
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
        Some(RustVerificationTraceMaxSeconds::new(45))
    );
}

#[test]
fn verification_report_writer_splits_source_baseline_from_runtime_cache() {
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
    let source_dir = root.join("resources/verification/reports");
    let cache_dir = root.join(".cache/agent/verification/sample");

    let receipt = write_rust_verification_reports(
        &plan,
        &RustVerificationReportWriteConfig::new(root, &source_dir, &cache_dir),
    )
    .expect("write reports");

    assert!(
        source_dir
            .join("verification_report_manifest.json")
            .exists()
    );
    assert!(source_dir.join("performance_index.json").exists());
    assert!(source_dir.join("task_index.json").exists());
    assert!(!source_dir.join("verification_plan.json").exists());
    assert!(cache_dir.join("verification_report_manifest.json").exists());
    assert!(cache_dir.join("verification_plan.json").exists());
    assert_eq!(receipt.source_baseline_paths.len(), 3);
    assert_eq!(receipt.runtime_cache_paths.len(), 2);

    let source_manifest =
        std::fs::read_to_string(source_dir.join("verification_report_manifest.json"))
            .expect("source manifest");
    let cache_manifest =
        std::fs::read_to_string(cache_dir.join("verification_report_manifest.json"))
            .expect("cache manifest");
    let performance_index = std::fs::read_to_string(source_dir.join("performance_index.json"))
        .expect("performance index");

    assert!(source_manifest.contains("performance_index_json"));
    assert!(source_manifest.contains("task_index_json"));
    assert!(!source_manifest.contains("verification_plan_json"));
    assert!(cache_manifest.contains("verification_plan_json"));
    assert!(cache_manifest.contains("task_index_json"));
    assert!(cache_manifest.contains("performance_index_json"));
    assert!(performance_index.contains("$CRATE_ROOT"));
    assert!(!performance_index.contains(&root.display().to_string()));
}

#[test]
fn verification_report_writer_compacts_windows_json_escaped_project_root() {
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
    let mut plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let windows_root = PathBuf::from(r"C:\agent\work\verification-api");
    let windows_api = PathBuf::from(r"C:\agent\work\verification-api\src\api.rs");
    plan.project_root = windows_root.clone();
    for task in &mut plan.tasks {
        task.package_root = windows_root.clone();
        task.owner_path = windows_api.clone();
    }

    let source_dir = root.join("resources/verification/reports");
    let cache_dir = root.join(".cache/agent/verification/sample");
    write_rust_verification_reports(
        &plan,
        &RustVerificationReportWriteConfig::new(&windows_root, &source_dir, &cache_dir),
    )
    .expect("write reports");

    let performance_index = std::fs::read_to_string(source_dir.join("performance_index.json"))
        .expect("performance index");

    assert!(performance_index.contains("$CRATE_ROOT"));
    assert!(!performance_index.contains(r"C:\agent\work"));
    assert!(!performance_index.contains(r"C:\\agent\\work"));
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
