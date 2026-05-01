use std::fs;
use std::path::Path;

use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationPolicy, RustVerificationProfileHint,
    RustVerificationReceipt, RustVerificationTaskKind, RustVerificationTaskState,
    RustVerificationWaiver, default_rust_harness_config,
    plan_rust_project_verification_with_config, render_rust_verification_plan,
};
use tempfile::TempDir;

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
    let rendered = render_rust_verification_plan(&failed_plan);

    assert!(!failed_plan.is_clear());
    assert!(rendered.contains("[verify:stress] failed"), "{rendered}");
    assert!(
        rendered.contains("p99 exceeded SLA at step 4"),
        "{rendered}"
    );
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

fn public_api_profile_config() -> rust_lang_project_harness::RustHarnessConfig {
    default_rust_harness_config().with_verification_profile_hint(RustVerificationProfileHint::new(
        "src/api.rs",
        [RustOwnerResponsibility::PublicApi],
    ))
}

fn write_manifest(root: &Path, name: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("write manifest");
}

fn write_api_project(root: &Path) {
    write_manifest(root, "verification-api");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! API owner.\npub fn handle_request() {}\n",
    )
    .expect("write api");
}

fn write_external_dependency_project(root: &Path) {
    write_manifest(root, "verification-external");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain owner.\nuse std::fs;\npub fn read_state() {}\n",
    )
    .expect("write domain");
}

fn write_branch_project(root: &Path) {
    write_manifest(root, "verification-branch");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain branch.\nmod alpha;\nmod beta;\nmod gamma;\n",
    )
    .expect("write domain");
    fs::write(root.join("src/domain/alpha.rs"), "//! Alpha.\n").expect("write alpha");
    fs::write(root.join("src/domain/beta.rs"), "//! Beta.\n").expect("write beta");
    fs::write(root.join("src/domain/gamma.rs"), "//! Gamma.\n").expect("write gamma");
}

fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}
