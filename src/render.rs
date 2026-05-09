//! Agent-first compact text rendering for Rust harness diagnostics.

use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::Path;

use crate::{RustDiagnosticSeverity, RustHarnessFinding, RustHarnessReport};

/// Render a compact diagnostic report with advice enabled.
#[must_use]
pub fn render_rust_project_harness(report: &RustHarnessReport) -> String {
    render_rust_project_harness_with_options(report, None, true)
}

/// Render a structured JSON diagnostic report for tool consumers.
///
/// This is the library equivalent of the CLI `--json` output mode.
///
/// # Errors
///
/// Returns a serialization error if the report cannot be encoded as JSON.
pub fn render_rust_project_harness_json(
    report: &RustHarnessReport,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(report)
}

/// Render only non-blocking agent advice.
#[must_use]
pub fn render_rust_project_harness_advice(report: &RustHarnessReport) -> String {
    let severities = BTreeSet::from([RustDiagnosticSeverity::Info]);
    render_finding_list(&report.blocking_findings(Some(&severities)))
}

/// Render a compact diagnostic report with explicit severity and advice options.
#[must_use]
pub fn render_rust_project_harness_with_options(
    report: &RustHarnessReport,
    severities: Option<&BTreeSet<RustDiagnosticSeverity>>,
    include_advice: bool,
) -> String {
    let blocking_findings = report.blocking_findings(severities);
    let advice = if include_advice {
        deduplicate_advice_findings(&report.advisory_findings(), &blocking_findings)
    } else {
        Vec::new()
    };
    let findings = blocking_findings
        .iter()
        .chain(advice.iter())
        .copied()
        .collect::<Vec<_>>();
    if findings.is_empty() {
        return "[ok] rust\n".to_string();
    }
    render_finding_list(&findings)
}

fn render_finding(finding: &RustHarnessFinding) -> String {
    let path = finding
        .location
        .path
        .as_ref()
        .map_or_else(|| "<memory>".to_string(), |path| display_path(path));
    let display_column = finding.location.column + 1;
    let mut rendered = format!(
        "[{}] {}: {}\n@ {path}:{}:{display_column}\n",
        finding.rule_id,
        title_case(finding.severity.as_str()),
        finding.title,
        finding.location.line
    );
    let _ = writeln!(rendered, "fix: {}", finding.label);
    if let Some(source_line) = &finding.source_line {
        let _ = writeln!(
            rendered,
            "line: {} | {}",
            finding.location.line, source_line
        );
    }
    let _ = writeln!(rendered, "Help: {}", finding.summary);
    let _ = writeln!(rendered, "Contract: {}", finding.requirement);
    rendered
}

fn deduplicate_advice_findings<'a>(
    advice_findings: &[&'a RustHarnessFinding],
    blocking_findings: &[&RustHarnessFinding],
) -> Vec<&'a RustHarnessFinding> {
    let blocking_keys = blocking_findings
        .iter()
        .map(|finding| finding_key(finding))
        .collect::<BTreeSet<_>>();
    advice_findings
        .iter()
        .copied()
        .filter(|finding| !blocking_keys.contains(&finding_key(finding)))
        .collect()
}

fn finding_key(finding: &RustHarnessFinding) -> (String, Option<String>, usize, usize) {
    (
        finding.rule_id.clone(),
        finding
            .location
            .path
            .as_ref()
            .map(|path| display_path(path)),
        finding.location.line,
        finding.location.column,
    )
}

fn render_finding_list(findings: &[&RustHarnessFinding]) -> String {
    findings
        .iter()
        .map(|finding| render_finding(finding))
        .collect::<Vec<_>>()
        .join("\n")
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    first.to_uppercase().collect::<String>() + chars.as_str()
}

fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}
