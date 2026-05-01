use std::fs;
use std::path::PathBuf;

use rust_lang_project_harness::{
    RustDiagnosticSeverity, default_rust_harness_config, render_rust_project_harness,
    render_rust_project_harness_advice, render_rust_project_harness_json, run_rust_lang_harness,
    run_rust_project_harness,
};
use tempfile::TempDir;

mod embedded_cargo_test_gate_macro_smoke {
    rust_lang_project_harness::rust_project_harness_cargo_test_gate!();
}

#[test]
fn explicit_path_runner_returns_compact_report() {
    let paths = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")];

    let report = run_rust_lang_harness(&paths).expect("run harness over lib.rs");

    assert_eq!(report.file_count(), 1);
    assert!(report.parsed_count() == 1);
    let rendered = render_rust_project_harness(&report);
    assert!(rendered.contains("[ok]"));
    assert!(rendered.contains("No blocking issues found."));
}

#[test]
fn explicit_path_runner_is_syntax_only_without_project_scope() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("lib.rs");
    fs::write(
        &source,
        "pub fn public_api() {}\n#[cfg(test)] mod tests {}\n",
    )
    .expect("write source");

    let report = run_rust_lang_harness(&[source]).expect("run explicit path harness");

    assert!(report.is_clean());
    assert!(report.project_scope.is_none());
    assert!(report.findings.is_empty());
}

#[test]
fn explicit_path_runner_reports_unreadable_source_as_syntax_error() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("invalid_utf8.rs");
    fs::write(&source, [0xff]).expect("write invalid utf8");

    let report = run_rust_lang_harness(&[source]).expect("run explicit path harness");
    let rendered = render_rust_project_harness(&report);

    assert_eq!(report.file_count(), 1);
    assert_eq!(report.parsed_count(), 0);
    assert!(
        report
            .modules
            .first()
            .and_then(|module| module.parse_error.as_deref())
            .is_some_and(|error| error.contains("failed to read Rust source")),
        "{rendered}"
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == "RUST-SYN-R001"),
        "{rendered}"
    );
}

#[test]
fn advice_renderer_selects_info_findings() {
    let config = default_rust_harness_config();
    assert!(
        config
            .blocking_severities
            .contains(&RustDiagnosticSeverity::Warning)
    );

    let paths = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")];
    let report = run_rust_lang_harness(&paths).expect("run harness over lib.rs");
    let rendered = render_rust_project_harness_advice(&report);

    assert!(rendered.contains("rust"));
}

#[test]
fn default_renderer_keeps_info_advice_visible_without_blocking() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"advice-only\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "mod owned;\npub use owned::public_api;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/owned.rs"), "pub fn public_api() {}\n").expect("write owned module");

    let report = run_rust_project_harness(root).expect("run project harness");
    let rendered = render_rust_project_harness(&report);

    assert!(report.is_clean(), "{rendered}");
    assert!(rendered.contains("[advice]"));
    assert!(rendered.contains("AGENT-R001"));
    assert!(rendered.contains("AGENT-R002"));
    assert!(rendered.contains("Help:"));
    assert!(rendered.contains("Contract:"));
}

#[test]
fn json_renderer_preserves_structured_report_fields() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"json-output\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "mod owned;\npub use owned::public_api;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/owned.rs"), "pub fn public_api() {}\n").expect("write owned module");

    let report = run_rust_project_harness(root).expect("run project harness");
    let json = render_rust_project_harness_json(&report).expect("render json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");

    assert_eq!(value["findings"][0]["rule_id"], "AGENT-R001");
    assert!(value["findings"][0]["summary"].as_str().is_some());
    assert!(value["findings"][0]["requirement"].as_str().is_some());
    assert!(value["project_scope"].is_object());
}
