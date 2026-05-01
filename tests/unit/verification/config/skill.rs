use rust_lang_project_harness::{
    RustVerificationSkillBinding, RustVerificationTaskKind,
    plan_rust_project_verification_with_config, render_rust_verification_plan,
    render_rust_verification_plan_json,
};
use tempfile::TempDir;

use crate::verification::support::{
    normalize_temp_root, public_api_profile_config, write_api_project,
};

#[test]
fn verification_skill_binding_renders_quiet_dispatch_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = public_api_profile_config().with_verification_skill_binding(
        RustVerificationTaskKind::Stress,
        RustVerificationSkillBinding::new("rust-verification-stress").with_adapter("k6"),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = plan.active_tasks()[0];
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(task.kind, RustVerificationTaskKind::Stress);
    assert_eq!(
        task.skill_binding.as_ref().expect("skill binding").skill_id,
        "rust-verification-stress"
    );
    assert!(
        rendered.contains("skill=rust-verification-stress@k6"),
        "{rendered}"
    );
    assert!(!rendered.contains("|why:"), "{rendered}");
    assert!(!rendered.contains("|requires:"), "{rendered}");
    assert!(!rendered.contains("|fact:"), "{rendered}");
    assert!(!rendered.contains("|contract:"), "{rendered}");
    insta::assert_snapshot!(
        "verification_configured_skill_binding_stays_quiet",
        rendered
    );
}

#[test]
fn verification_skill_binding_remains_structured_json() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = public_api_profile_config().with_verification_skill_binding(
        RustVerificationTaskKind::Stress,
        RustVerificationSkillBinding::new("rust-verification-stress"),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let json = render_rust_verification_plan_json(&plan).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");

    assert_eq!(
        value["tasks"][0]["skill_binding"]["skill_id"],
        "rust-verification-stress"
    );
    assert_eq!(value["tasks"][0]["evidence"][1]["label"], "skill");
    assert_eq!(
        value["tasks"][0]["evidence"][1]["value"],
        "rust-verification-stress"
    );
    assert_eq!(value["tasks"][0]["required_evidence"][0]["key"], "p50");
}
