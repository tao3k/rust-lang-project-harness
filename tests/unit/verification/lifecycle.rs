use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationPolicy, RustVerificationProfileHint,
    RustVerificationReceipt, RustVerificationTaskKind, RustVerificationTaskState,
    RustVerificationWaiver, default_rust_harness_config,
    plan_rust_project_verification_with_config, render_rust_verification_plan,
    render_rust_verification_plan_json,
};
use tempfile::TempDir;

use super::support::{
    normalize_temp_root, public_api_profile_config, write_api_project, write_branch_project,
    write_external_dependency_project,
};

#[test]
fn verification_profile_tasks_render_compact_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_rust_harness_config().with_verification_profile_hint(
        RustVerificationProfileHint::new(
            "src/api.rs",
            [
                RustOwnerResponsibility::PublicApi,
                RustOwnerResponsibility::LatencySensitive,
                RustOwnerResponsibility::ExternalDependency,
                RustOwnerResponsibility::SecurityBoundary,
            ],
        ),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 3, "{rendered}");
    insta::assert_snapshot!("verification_profile_tasks", rendered);
}

#[test]
fn verification_receipt_clears_matching_task() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = public_api_profile_config();
    let initial_plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = initial_plan
        .active_tasks()
        .into_iter()
        .find(|task| task.kind == RustVerificationTaskKind::Stress)
        .expect("stress task");
    let resolved_config = public_api_profile_config().with_verification_receipt(
        RustVerificationReceipt::passed(task.fingerprint.clone(), RustVerificationTaskKind::Stress),
    );

    let resolved_plan =
        plan_rust_project_verification_with_config(root, &resolved_config).expect("resolved plan");
    let stress = resolved_plan
        .tasks
        .iter()
        .find(|task| task.kind == RustVerificationTaskKind::Stress)
        .expect("stress task");

    assert_eq!(stress.state, RustVerificationTaskState::Satisfied);
    assert!(resolved_plan.is_clear());
    assert_eq!(render_rust_verification_plan(&resolved_plan), "");
}

#[test]
fn failed_verification_receipt_remains_active() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = public_api_profile_config();
    let initial_plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = initial_plan.active_tasks()[0];
    let failed_config =
        public_api_profile_config().with_verification_receipt(RustVerificationReceipt::failed(
            task.fingerprint.clone(),
            RustVerificationTaskKind::Stress,
            "p99 exceeded SLA at step 4",
        ));

    let failed_plan =
        plan_rust_project_verification_with_config(root, &failed_config).expect("failed plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&failed_plan), root);

    assert!(!failed_plan.is_clear());
    assert!(rendered.contains("|stress: failed"), "{rendered}");
    assert!(
        rendered.contains("p99 exceeded SLA at step 4"),
        "{rendered}"
    );
    insta::assert_snapshot!("verification_failed_receipt_resolution", rendered);
}

#[test]
fn complete_verification_waiver_clears_matching_task() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = public_api_profile_config();
    let initial_plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = initial_plan.active_tasks()[0];
    let waived_config =
        public_api_profile_config().with_verification_waiver(RustVerificationWaiver::new(
            task.fingerprint.clone(),
            "platform",
            "covered by upstream gateway test for this release",
            "2026-06-01",
        ));

    let waived_plan =
        plan_rust_project_verification_with_config(root, &waived_config).expect("waived plan");
    let stress = waived_plan
        .tasks
        .iter()
        .find(|task| task.kind == RustVerificationTaskKind::Stress)
        .expect("stress task");

    assert_eq!(stress.state, RustVerificationTaskState::Waived);
    assert!(waived_plan.is_clear());
    assert_eq!(render_rust_verification_plan(&waived_plan), "");
}

#[test]
fn incomplete_verification_waiver_keeps_task_active_with_resolution_note() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = public_api_profile_config();
    let initial_plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = initial_plan.active_tasks()[0];
    let incomplete_config = public_api_profile_config().with_verification_waiver(
        RustVerificationWaiver::new(task.fingerprint.clone(), "", "", ""),
    );

    let plan = plan_rust_project_verification_with_config(root, &incomplete_config).expect("plan");
    let stress = plan
        .tasks
        .iter()
        .find(|task| task.kind == RustVerificationTaskKind::Stress)
        .expect("stress task");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(stress.state, RustVerificationTaskState::Pending);
    assert!(!plan.is_clear());
    assert!(
        rendered
            .contains("resolution: stress.waiver=incomplete: missing owner, reason, expires_at"),
        "{rendered}"
    );
    insta::assert_snapshot!("verification_incomplete_waiver_resolution", rendered);
}

#[test]
fn parser_facts_can_reject_wrong_responsibility_profile() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_external_dependency_project(root);
    let config = default_rust_harness_config().with_verification_profile_hint(
        RustVerificationProfileHint::new(
            "src/domain.rs",
            [RustOwnerResponsibility::PureDomainLogic],
        ),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 1, "{rendered}");
    assert_eq!(
        plan.active_tasks()[0].kind,
        RustVerificationTaskKind::ResponsibilityReview
    );
    insta::assert_snapshot!("verification_profile_conflict", rendered);
}

#[test]
fn parser_facts_generate_regression_task_for_large_owner_branch() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_branch_project(root);

    let plan = plan_rust_project_verification_with_config(root, &default_rust_harness_config())
        .expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 1, "{rendered}");
    assert_eq!(
        plan.active_tasks()[0].kind,
        RustVerificationTaskKind::Regression
    );
    insta::assert_snapshot!("verification_parser_regression_task", rendered);
}

#[test]
fn verification_policy_can_disable_task_kind() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let policy = RustVerificationPolicy::default()
        .with_profile_hint(RustVerificationProfileHint::new(
            "src/api.rs",
            [RustOwnerResponsibility::PublicApi],
        ))
        .with_disabled_task_kind(RustVerificationTaskKind::Stress);
    let config = default_rust_harness_config().with_verification_policy(policy);

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");

    assert!(plan.tasks.is_empty(), "{plan:?}");
}

#[test]
fn verification_json_preserves_structured_plan_fields() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let plan = plan_rust_project_verification_with_config(root, &public_api_profile_config())
        .expect("plan");

    let json = render_rust_verification_plan_json(&plan).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");

    assert!(value["project_root"].as_str().is_some());
    assert_eq!(value["tasks"][0]["kind"], "stress");
    assert_eq!(value["tasks"][0]["state"], "pending");
    assert_eq!(value["tasks"][0]["required_evidence"][0]["key"], "p50");
    assert_eq!(
        value["tasks"][0]["required_evidence"][4]["key"],
        "sla_result"
    );
    assert_eq!(value["tasks"][0]["owner_namespace"][0], "src");
    assert_eq!(value["tasks"][0]["owner_namespace"][1], "api");
    assert!(value["tasks"][0]["resolution_notes"].is_null());
}
