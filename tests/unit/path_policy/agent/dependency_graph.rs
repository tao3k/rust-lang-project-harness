use std::fs;

use rust_lang_project_harness::run_rust_project_harness;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn owner_dependency_cycle_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "owner-cycle");
    write_owner_cycle_fixture(root);

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "AGENT-R009");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("src/alpha.rs"));
    assert!(findings[0].summary.contains("src/beta.rs"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn cross_owner_leaf_import_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cross-owner-leaf");
    write_cross_owner_leaf_fixture(root);

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "AGENT-R010");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("src/ui.rs"));
    assert!(findings[0].summary.contains("src/domain/leaf.rs"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn owner_fan_out_without_intent_doc_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "owner-fan-out");
    write_owner_fan_out_fixture(root);

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "AGENT-R011");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("src/orchestrator.rs"));
    assert!(findings[0].summary.contains("3 owner branches"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn cfg_test_owner_dependencies_do_not_trigger_structural_agent_policies() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "test-context-owner-deps");
    fs::create_dir_all(root.join("src/alpha")).expect("create alpha dir");
    fs::create_dir_all(root.join("src/beta")).expect("create beta dir");
    fs::create_dir_all(root.join("src/gamma")).expect("create gamma dir");
    fs::create_dir_all(root.join("src/orchestrator")).expect("create orchestrator dir");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod alpha;\nmod beta;\nmod gamma;\nmod orchestrator;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/alpha.rs"),
        "//! Alpha owner.\nmod core;\n#[cfg(test)]\nmod tests {\n    use crate::beta::Beta;\n}\npub struct Alpha;\n",
    )
    .expect("write alpha");
    fs::write(
        root.join("src/beta.rs"),
        "//! Beta owner.\nmod core;\n#[cfg(test)]\nmod tests {\n    use crate::alpha::Alpha;\n}\npub struct Beta;\n",
    )
    .expect("write beta");
    fs::write(
        root.join("src/gamma.rs"),
        "//! Gamma owner.\nmod core;\npub struct Gamma;\n",
    )
    .expect("write gamma");
    fs::write(
        root.join("src/orchestrator.rs"),
        "mod task;\n#[cfg(test)]\nmod tests {\n    use crate::alpha::Alpha;\n    use crate::beta::Beta;\n    use crate::gamma::Gamma;\n}\n",
    )
    .expect("write orchestrator");
    fs::write(root.join("src/alpha/core.rs"), "//! Alpha core.\n").expect("write alpha core");
    fs::write(root.join("src/beta/core.rs"), "//! Beta core.\n").expect("write beta core");
    fs::write(root.join("src/gamma/core.rs"), "//! Gamma core.\n").expect("write gamma core");
    fs::write(root.join("src/orchestrator/task.rs"), "//! Task.\n").expect("write task");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(findings_for_rule(&report, "AGENT-R009").is_empty());
    assert!(findings_for_rule(&report, "AGENT-R011").is_empty());
}

fn write_owner_cycle_fixture(root: &std::path::Path) {
    fs::create_dir_all(root.join("src/alpha")).expect("create alpha dir");
    fs::create_dir_all(root.join("src/beta")).expect("create beta dir");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod alpha;\nmod beta;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/alpha.rs"),
        "//! Alpha owner.\nuse crate::beta::Beta;\nmod core;\npub struct Alpha;\n",
    )
    .expect("write alpha");
    fs::write(
        root.join("src/beta.rs"),
        "//! Beta owner.\nuse crate::alpha::Alpha;\nmod core;\npub struct Beta;\n",
    )
    .expect("write beta");
    fs::write(root.join("src/alpha/core.rs"), "//! Alpha core.\n").expect("write alpha core");
    fs::write(root.join("src/beta/core.rs"), "//! Beta core.\n").expect("write beta core");
}

fn write_cross_owner_leaf_fixture(root: &std::path::Path) {
    fs::create_dir_all(root.join("src/domain")).expect("create domain dir");
    fs::create_dir_all(root.join("src/ui")).expect("create ui dir");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod domain;\nmod ui;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/domain.rs"), "//! Domain owner.\nmod leaf;\n").expect("write domain");
    fs::write(
        root.join("src/domain/leaf.rs"),
        "//! Domain leaf.\npub struct Leaf;\n",
    )
    .expect("write leaf");
    fs::write(
        root.join("src/ui.rs"),
        "//! Ui owner.\nuse crate::domain::leaf::Leaf;\nmod view;\n",
    )
    .expect("write ui");
    fs::write(root.join("src/ui/view.rs"), "//! Ui view.\n").expect("write view");
}

fn write_owner_fan_out_fixture(root: &std::path::Path) {
    fs::create_dir_all(root.join("src/alpha")).expect("create alpha dir");
    fs::create_dir_all(root.join("src/beta")).expect("create beta dir");
    fs::create_dir_all(root.join("src/gamma")).expect("create gamma dir");
    fs::create_dir_all(root.join("src/orchestrator")).expect("create orchestrator dir");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod alpha;\nmod beta;\nmod gamma;\nmod orchestrator;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/alpha.rs"),
        "//! Alpha owner.\nmod core;\npub struct Alpha;\n",
    )
    .expect("write alpha");
    fs::write(
        root.join("src/beta.rs"),
        "//! Beta owner.\nmod core;\npub struct Beta;\n",
    )
    .expect("write beta");
    fs::write(
        root.join("src/gamma.rs"),
        "//! Gamma owner.\nmod core;\npub struct Gamma;\n",
    )
    .expect("write gamma");
    fs::write(
        root.join("src/orchestrator.rs"),
        "use crate::alpha::Alpha;\nuse crate::beta::Beta;\nuse crate::gamma::Gamma;\nmod task;\n",
    )
    .expect("write orchestrator");
    fs::write(root.join("src/alpha/core.rs"), "//! Alpha core.\n").expect("write alpha core");
    fs::write(root.join("src/beta/core.rs"), "//! Beta core.\n").expect("write beta core");
    fs::write(root.join("src/gamma/core.rs"), "//! Gamma core.\n").expect("write gamma core");
    fs::write(root.join("src/orchestrator/task.rs"), "//! Task.\n").expect("write task");
}
