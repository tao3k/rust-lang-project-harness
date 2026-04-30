use std::fs;

use tempfile::TempDir;
use xiuxian_harness_rust_lang_project::run_rust_project_harness;

use super::support::{has_rule, write_manifest};

#[test]
fn source_test_policy_does_not_treat_latest_feature_as_cfg_test() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cfg-feature-latest-source");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(feature = \"latest\")]\nmod optional;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/optional.rs"), "//! Optional owner.\n").expect("write optional");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-PROJ-R003"),
        "{:?}",
        report.findings
    );
    assert!(
        !has_rule(&report, "RUST-PROJ-R004"),
        "{:?}",
        report.findings
    );
}

#[test]
fn root_test_target_accepts_embedded_cargo_test_gate_macro() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "embedded-gate-target");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir(root.join("tests")).expect("create tests");
    fs::write(
        root.join("tests/unit_test.rs"),
        "xiuxian_harness_rust_lang_project::rust_project_harness_cargo_test_gate!();\n",
    )
    .expect("write root test target");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-PROJ-R006"),
        "{:?}",
        report.findings
    );
}

#[test]
fn library_target_requires_cargo_test_gate_when_harness_is_dev_dependency() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "missing-embedded-lib-gate");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"missing-embedded-lib-gate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies]\nxiuxian-harness-rust-lang-project = { path = \".\" }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(has_rule(&report, "RUST-PROJ-R009"), "{:?}", report.findings);
}

#[test]
fn library_target_ignores_comment_mentions_of_embedded_cargo_test_gate() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"comment-mention-lib-gate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies]\nxiuxian-harness-rust-lang-project = { path = \".\" }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Mentioning rust_project_harness_cargo_test_gate!() here is not a gate.\nconst NOTE: &str = \"rust_project_harness_cargo_test_gate!()\";\n",
    )
    .expect("write lib");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(has_rule(&report, "RUST-PROJ-R009"), "{:?}", report.findings);
}

#[test]
fn library_target_accepts_embedded_cargo_test_gate() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"embedded-lib-gate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies]\nxiuxian-harness-rust-lang-project = { path = \".\" }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)]\nxiuxian_harness_rust_lang_project::rust_project_harness_cargo_test_gate!();\n",
    )
    .expect("write lib");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-PROJ-R009"),
        "{:?}",
        report.findings
    );
}
