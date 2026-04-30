//! Compact text rendering for Rust harness diagnostics.

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
    render_rust_project_harness_with_options(report, Some(&severities), false)
}

/// Render a compact diagnostic report with explicit severity and advice options.
#[must_use]
pub fn render_rust_project_harness_with_options(
    report: &RustHarnessReport,
    severities: Option<&BTreeSet<RustDiagnosticSeverity>>,
    include_advice: bool,
) -> String {
    let blocking_findings = report.blocking_findings(severities);
    let mut rendered = render_header(report, &blocking_findings);
    for finding in &blocking_findings {
        rendered.push('\n');
        rendered.push_str(&render_finding(finding));
    }
    if include_advice {
        let advice = deduplicate_advice_findings(&report.advisory_findings(), &blocking_findings);
        if !advice.is_empty() {
            let _ = writeln!(rendered, "\n[advice]");
            let _ = writeln!(rendered, "Issues: {}", advice.len());
            for finding in advice {
                rendered.push('\n');
                rendered.push_str(&render_finding(finding));
            }
        }
    }
    rendered
}

fn render_header(report: &RustHarnessReport, blocking_findings: &[&RustHarnessFinding]) -> String {
    let target = report
        .root_paths
        .iter()
        .map(|path| display_path(path))
        .collect::<Vec<_>>()
        .join(", ");
    if blocking_findings.is_empty() {
        return format!(
            "[ok] {target} rust\nSource: {target}\nFiles: {} Parsed: {}\nNo blocking issues found.\n",
            report.file_count(),
            report.parsed_count()
        );
    }
    format!(
        "[lint:{}] {target} rust\nSource: {target}\nFiles: {} Parsed: {}\nIssues: {}\n",
        findings_status(blocking_findings),
        report.file_count(),
        report.parsed_count(),
        blocking_findings.len()
    )
}

fn render_finding(finding: &RustHarnessFinding) -> String {
    let path = finding
        .location
        .path
        .as_ref()
        .map_or_else(|| "<memory>".to_string(), |path| display_path(path));
    let display_column = finding.location.column + 1;
    let mut rendered = format!(
        "[{}] {}: {}\n   ,-[ {path}:{}:{display_column} ]\n",
        finding.rule_id,
        title_case(finding.severity.as_str()),
        finding.title,
        finding.location.line
    );
    if let Some(source_line) = &finding.source_line {
        let pointer_column = finding.location.column;
        let _ = writeln!(
            rendered,
            "{:>2} | {}\n   | {}`- {}",
            finding.location.line,
            source_line,
            " ".repeat(pointer_column),
            finding.label
        );
    } else {
        let _ = writeln!(rendered, "   | {}", finding.label);
    }
    let _ = writeln!(rendered, "   |Help: {}", finding.summary);
    let _ = writeln!(rendered, "   |Contract: {}", finding.requirement);
    rendered
}

fn findings_status(findings: &[&RustHarnessFinding]) -> &'static str {
    if findings
        .iter()
        .any(|finding| finding.severity == RustDiagnosticSeverity::Error)
    {
        return "error";
    }
    if findings
        .iter()
        .any(|finding| finding.severity == RustDiagnosticSeverity::Warning)
    {
        return "warning";
    }
    "info"
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
