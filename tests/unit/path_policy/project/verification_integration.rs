use std::fs;

use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationProfileHint, RustVerificationSkillBinding,
    RustVerificationSkillDescriptor, RustVerificationTaskKind, default_rust_harness_config,
    run_rust_project_harness_with_config,
};
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn performance_verification_binding_requires_cargo_bench_target() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "missing-performance-bench");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)]\nrust_lang_project_harness::rust_project_harness_cargo_test_gate!();\nmod api;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/api.rs"), "//! API.\npub fn load() {}\n").expect("write api");

    let report = run_rust_project_harness_with_config(root, &performance_config())
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-PROJ-R010");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0].summary.contains("harness=false [[bench]]"),
        "{:?}",
        findings[0]
    );
}

#[test]
fn performance_verification_binding_accepts_cargo_bench_target() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"wired-performance-bench\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bench]]\nname = \"api_perf\"\nharness = false\nrequired-features = [\"performance\"]\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)]\nrust_lang_project_harness::rust_project_harness_cargo_test_gate!();\nmod api;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/api.rs"), "//! API.\npub fn load() {}\n").expect("write api");
    fs::create_dir(root.join("benches")).expect("create benches");
    fs::write(root.join("benches/api_perf.rs"), "fn main() {}\n").expect("write bench");

    let report = run_rust_project_harness_with_config(root, &performance_config())
        .expect("run project harness");

    assert!(
        findings_for_rule(&report, "RUST-PROJ-R010").is_empty(),
        "{:?}",
        report.findings
    );
}

fn performance_config() -> rust_lang_project_harness::RustHarnessConfig {
    default_rust_harness_config()
        .with_verification_profile_hint(RustVerificationProfileHint::new(
            "src/api.rs",
            [RustOwnerResponsibility::LatencySensitive],
        ))
        .with_verification_skill_binding(
            RustVerificationTaskKind::Performance,
            RustVerificationSkillBinding::new("rust-verification-performance")
                .with_adapter("criterion"),
        )
        .with_verification_skill_descriptor(
            RustVerificationSkillDescriptor::criterion_performance(),
        )
}
