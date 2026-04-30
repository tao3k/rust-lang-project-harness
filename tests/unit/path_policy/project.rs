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
