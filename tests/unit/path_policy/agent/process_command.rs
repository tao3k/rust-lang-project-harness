use std::fs;

use rust_lang_project_harness::run_rust_project_harness_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn raw_skill_file_source_dump_process_command_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "process-command-probe");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "mod process_command_probe;\n").expect("write lib");
    fs::write(
        root.join("src/process_command_probe.rs"),
        r#"use std::process::Command;

pub fn process_command_probe() {
    let _ = Command::new("sed")
        .args([
            "-n",
            "1,220p",
            "/Users/guangtao/.agents/skills/brainstorming/SKILL.md",
        ])
        .status();
}
"#,
    )
    .expect("write process command probe");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    let findings = findings_for_rule(&report, "RUST-AGENT-PROC-001");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(report.is_clean(), "{:?}", report.findings);
}
