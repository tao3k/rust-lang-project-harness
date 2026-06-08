use std::fs;
use std::path::{Path, PathBuf};

use rust_lang_project_harness::{
    RustHarnessFinding, render_rust_project_harness, run_rust_project_harness,
};
use serde::Serialize;
use tempfile::TempDir;

const SCENARIO: &str = "tests/unit/scenarios/software_criteria/control_flow_v1";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FindingSnapshot {
    rule_id: String,
    summary: String,
    line: usize,
    label: String,
    software_criteria: Option<String>,
}

#[test]
fn agent_r015_control_flow_v1_snapshot() {
    insta::assert_snapshot!("agent_r015_control_flow_v1", rule_snapshot("AGENT-R015"));
}

#[test]
fn agent_r016_control_flow_v1_snapshot() {
    insta::assert_snapshot!("agent_r016_control_flow_v1", rule_snapshot("AGENT-R016"));
}

#[test]
fn agent_r017_control_flow_v1_snapshot() {
    insta::assert_snapshot!("agent_r017_control_flow_v1", rule_snapshot("AGENT-R017"));
}

#[test]
fn agent_r025_control_flow_v1_snapshot() {
    insta::assert_snapshot!("agent_r025_control_flow_v1", rule_snapshot("AGENT-R025"));
}

#[test]
fn agent_r026_control_flow_v1_snapshot() {
    insta::assert_snapshot!("agent_r026_control_flow_v1", rule_snapshot("AGENT-R026"));
}

fn rule_snapshot(rule_id: &str) -> String {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    copy_inputs(Path::new(SCENARIO).join("inputs"), root);

    let mut report = run_rust_project_harness(root).expect("run project harness");
    report.findings.retain(|finding| finding.rule_id == rule_id);
    let findings = report
        .findings
        .iter()
        .map(|finding| finding_snapshot(finding, root))
        .collect::<Vec<_>>();
    assert!(
        !findings.is_empty(),
        "expected {rule_id} finding in control_flow_v1 scenario"
    );
    let rendered = normalize_temp_root(&render_rust_project_harness(&report), root);

    let findings_json =
        serde_json::to_string_pretty(&findings).expect("serialize findings snapshot");
    format!("{findings_json}\n\n--- rendered ---\n{rendered}")
}

fn finding_snapshot(finding: &RustHarnessFinding, root: &Path) -> FindingSnapshot {
    FindingSnapshot {
        rule_id: finding.rule_id.clone(),
        summary: normalize_temp_root(&finding.summary, root),
        line: finding.location.line,
        label: finding.label.clone(),
        software_criteria: finding.labels.get("softwareCriteria").cloned(),
    }
}

fn copy_inputs(source_dir: impl AsRef<Path>, destination_dir: &Path) {
    for source in walk_files(source_dir.as_ref()) {
        let destination = destination_dir.join(
            source
                .strip_prefix(source_dir.as_ref())
                .expect("relative input path"),
        );
        fs::create_dir_all(destination.parent().expect("input parent"))
            .expect("create input parent");
        fs::copy(&source, &destination).expect("copy scenario input");
    }
}

fn walk_files(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .expect("read scenario inputs")
        .flat_map(|entry| {
            let path = entry.expect("scenario input entry").path();
            if path.is_dir() {
                walk_files(&path)
            } else {
                vec![path]
            }
        })
        .collect()
}

fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}
