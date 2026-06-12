use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationProfileHint, RustVerificationReceipt,
    RustVerificationStabilityPictureConfig, RustVerificationTaskKind,
    build_rust_verification_stability_picture, default_rust_harness_config,
    plan_rust_project_verification_with_config, render_rust_verification_stability_picture,
    render_rust_verification_stability_picture_json,
};
use tempfile::TempDir;

use crate::verification::support::{normalize_temp_root, write_api_project};

#[test]
fn stability_picture_renders_configured_agent_actions() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = availability_critical_config().with_verification_stability_picture(
        RustVerificationStabilityPictureConfig::new()
            .with_min_iterations(10_000)
            .with_min_duration_seconds(900),
    );
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let picture_config = config
        .verification_policy
        .stability_picture
        .as_ref()
        .expect("picture config");

    let picture = build_rust_verification_stability_picture(&plan, picture_config);
    let rendered = normalize_temp_root(&render_rust_verification_stability_picture(&picture), root);
    let record = picture.records.first().expect("picture record");

    assert_eq!(picture.records.len(), 1);
    assert_eq!(picture.actionable_records().len(), 1);
    assert_eq!(
        record.required_evidence_keys,
        [
            "stability_command",
            "iteration_window",
            "latency_distribution",
            "resource_delta",
            "state_growth",
            "determinism",
            "stability_artifact",
        ]
    );
    assert!(
        rendered.contains("[stability-picture] records=1 actionable=1"),
        "{rendered}"
    );
    assert!(rendered.contains("|min_iterations: 10000"), "{rendered}");
    assert!(
        rendered.contains("add long-running simulation receipt iterations>=10000 duration_s>=900"),
        "{rendered}"
    );
    assert!(
        rendered.contains("add performance interface latency distribution"),
        "{rendered}"
    );
}

#[test]
fn stability_picture_respects_downstream_axis_configuration() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = availability_critical_config().with_verification_stability_picture(
        RustVerificationStabilityPictureConfig::new()
            .with_performance_interface_required(false)
            .with_resource_delta_required(false)
            .with_state_growth_required(false),
    );
    let initial_plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = initial_plan
        .active_tasks()
        .into_iter()
        .find(|task| task.kind == RustVerificationTaskKind::Stability)
        .expect("stability task");
    let resolved_config =
        config.with_verification_receipt(partial_stability_receipt(&task.fingerprint));
    let plan = plan_rust_project_verification_with_config(root, &resolved_config).expect("plan");
    let picture_config = resolved_config
        .verification_policy
        .stability_picture
        .as_ref()
        .expect("picture config");

    let picture = build_rust_verification_stability_picture(&plan, picture_config);
    let record = picture.records.first().expect("picture record");

    assert_eq!(
        record.required_evidence_keys,
        [
            "stability_command",
            "iteration_window",
            "determinism",
            "stability_artifact",
        ]
    );
    assert!(record.missing_evidence_keys.is_empty(), "{record:?}");
    assert!(record.next_actions.is_empty(), "{record:?}");
}

#[test]
fn stability_picture_json_is_deterministic_for_same_plan() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = availability_critical_config().with_verification_stability_picture(
        RustVerificationStabilityPictureConfig::new().with_min_iterations(5000),
    );
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let picture_config = config
        .verification_policy
        .stability_picture
        .as_ref()
        .expect("picture config");

    let first = build_rust_verification_stability_picture(&plan, picture_config);
    let second = build_rust_verification_stability_picture(&plan, picture_config);
    let first_json = render_rust_verification_stability_picture_json(&first).expect("json");
    let second_json = render_rust_verification_stability_picture_json(&second).expect("json");

    assert_eq!(first_json, second_json);
    assert!(first_json.contains("\"min_iterations\":5000"));
}

fn availability_critical_config() -> rust_lang_project_harness::RustHarnessConfig {
    default_rust_harness_config().with_verification_profile_hint(RustVerificationProfileHint::new(
        "src/api.rs",
        [RustOwnerResponsibility::AvailabilityCritical],
    ))
}

fn partial_stability_receipt(fingerprint: &str) -> RustVerificationReceipt {
    RustVerificationReceipt::passed(fingerprint, RustVerificationTaskKind::Stability)
        .with_evidence(
            "stability_command",
            "cargo run --bin api-long-run -- --iterations 10000",
        )
        .with_evidence("iteration_window", "10000 iterations warmup=500 samples=20")
        .with_evidence("determinism", "20/20 replay fingerprints matched")
        .with_evidence("stability_artifact", "target/stability/api-long-run.json")
        .with_evidence_uri("target/stability/api-long-run.json")
}
