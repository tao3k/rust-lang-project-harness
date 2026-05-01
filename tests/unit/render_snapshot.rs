use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use rust_lang_project_harness::{
    RustDiagnosticSeverity, RustHarnessFinding, RustHarnessReport, RustModuleReport,
    SourceLocation, render_rust_project_harness, render_rust_project_harness_json,
};

#[test]
fn compact_text_render_matches_snapshot() {
    let rendered = render_rust_project_harness(&snapshot_report());

    assert_eq!(
        rendered,
        include_str!("snapshots/rust_project_harness_compact_text.snap")
    );
}

#[test]
fn json_render_matches_snapshot() {
    let rendered = render_rust_project_harness_json(&snapshot_report()).expect("render json");

    assert_eq!(
        rendered,
        include_str!("snapshots/rust_project_harness_json.snap").trim_end()
    );
}

fn snapshot_report() -> RustHarnessReport {
    let source_path = PathBuf::from("$TEMP/src/lib.rs");
    RustHarnessReport {
        modules: vec![RustModuleReport {
            path: source_path.clone(),
            is_valid: true,
            parse_error: None,
        }],
        findings: vec![RustHarnessFinding {
            rule_id: "RUST-PROJ-R003".to_string(),
            pack_id: "rust.project_policy".to_string(),
            severity: RustDiagnosticSeverity::Warning,
            title: "Inline source test module".to_string(),
            summary: "$TEMP/src/lib.rs contains inline #[cfg(test)] module `tests`.".to_string(),
            location: SourceLocation::new(Some(source_path), 5, 4),
            requirement: "Keep test bodies out of src files; mount source-backed unit tests from tests/unit with #[path].".to_string(),
            source_line: Some("    #[cfg(test)] mod tests { #[test] fn it_works() {} }".to_string()),
            label: "move this test module to tests/unit and mount it with #[path]".to_string(),
            labels: BTreeMap::from([
                ("domain".to_string(), "project-policy".to_string()),
                ("language".to_string(), "rust".to_string()),
            ]),
        }],
        root_paths: vec![PathBuf::from("$TEMP/src"), PathBuf::from("$TEMP/tests")],
        blocking_severities: BTreeSet::from([
            RustDiagnosticSeverity::Warning,
            RustDiagnosticSeverity::Error,
        ]),
        project_scope: None,
        workspace_member_scopes: Vec::new(),
    }
}
