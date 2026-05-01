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
fn verification_skill_binding_trigger_audit_snapshot() {
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
    let json = render_rust_verification_plan_json(&plan).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");
    let required_evidence = task
        .required_evidence
        .iter()
        .map(|requirement| requirement.key.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let trigger_evidence = task
        .evidence
        .iter()
        .map(|evidence| format!("{}={}", evidence.label, evidence.value))
        .collect::<Vec<_>>()
        .join(" ");
    let skill_binding = task.skill_binding.as_ref().expect("skill binding");
    let skill_label = skill_binding.adapter.as_ref().map_or_else(
        || skill_binding.skill_id.clone(),
        |adapter| format!("{}@{adapter}", skill_binding.skill_id),
    );
    let phase = value["tasks"][0]["phase"]
        .as_str()
        .expect("serialized phase");

    assert_eq!(
        value["tasks"][0]["skill_binding"]["skill_id"],
        "rust-verification-stress"
    );
    assert_eq!(value["tasks"][0]["evidence"][1]["label"], "skill");
    assert_eq!(
        value["tasks"][0]["evidence"][1]["value"],
        "rust-verification-stress@k6"
    );
    assert_eq!(value["tasks"][0]["required_evidence"][0]["key"], "p50");
    assert!(!rendered.contains("|why:"), "{rendered}");
    assert!(!rendered.contains("|requires:"), "{rendered}");
    assert!(!rendered.contains("|fact:"), "{rendered}");
    assert!(!rendered.contains("|contract:"), "{rendered}");
    let compact_audit = format!(
        "[verification-skill-trigger] src/api.rs\n   |{}: phase={phase} skill={skill_label}\n   |trigger: {trigger_evidence}\n   |requires: {required_evidence}\n   |quiet: omits=why,requires,fact,contract\n\n{rendered}",
        task.kind.as_str(),
    );
    insta::assert_snapshot!("verification_skill_binding_trigger_audit", compact_audit);
}
