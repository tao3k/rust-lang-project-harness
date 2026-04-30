//! Shared report model for Rust project harness runs.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Finding severity used by the Rust project harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RustDiagnosticSeverity {
    /// Non-blocking repair advice.
    Info,
    /// Blocking policy drift by default.
    Warning,
    /// Blocking syntax or structural failure.
    Error,
}

impl RustDiagnosticSeverity {
    /// Return a stable lowercase severity label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for RustDiagnosticSeverity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Source location for one finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    /// File path, when the finding is file-backed.
    pub path: Option<PathBuf>,
    /// One-based line number.
    pub line: usize,
    /// Zero-based column number.
    pub column: usize,
}

impl SourceLocation {
    /// Create a source location.
    #[must_use]
    pub fn new(path: Option<PathBuf>, line: usize, column: usize) -> Self {
        Self { path, line, column }
    }
}

/// Stable metadata for one rule pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RulePackDescriptor {
    /// Stable pack id.
    pub id: &'static str,
    /// Rule pack version.
    pub version: &'static str,
    /// Searchable domains for this pack.
    pub domains: &'static [&'static str],
    /// Default execution mode.
    pub default_mode: &'static str,
}

/// Compact rule metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RustHarnessRule {
    /// Stable rule id.
    pub rule_id: &'static str,
    /// Stable pack id.
    pub pack_id: &'static str,
    /// Rule severity.
    pub severity: RustDiagnosticSeverity,
    /// Short human title.
    pub title: &'static str,
    /// Precise requirement line.
    pub requirement: &'static str,
    /// Small labels for tooling.
    pub labels: BTreeMap<&'static str, &'static str>,
}

impl RustHarnessRule {
    /// Build one rule catalog entry.
    #[must_use]
    pub fn new(
        rule_id: &'static str,
        pack_id: &'static str,
        severity: RustDiagnosticSeverity,
        title: &'static str,
        requirement: &'static str,
        labels: BTreeMap<&'static str, &'static str>,
    ) -> Self {
        Self {
            rule_id,
            pack_id,
            severity,
            title,
            requirement,
            labels,
        }
    }
}

/// One deterministic harness finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustHarnessFinding {
    /// Stable rule id.
    pub rule_id: String,
    /// Stable pack id.
    pub pack_id: String,
    /// Finding severity.
    pub severity: RustDiagnosticSeverity,
    /// Short title.
    pub title: String,
    /// Concrete finding summary.
    pub summary: String,
    /// Source location.
    pub location: SourceLocation,
    /// Required repair contract.
    pub requirement: String,
    /// Source line at the location, when available.
    pub source_line: Option<String>,
    /// Short pointer label.
    pub label: String,
    /// Small labels for tooling.
    pub labels: BTreeMap<String, String>,
}

impl RustHarnessFinding {
    /// Build a finding from a catalog rule.
    #[must_use]
    pub fn from_rule(
        rule: &RustHarnessRule,
        summary: impl Into<String>,
        location: SourceLocation,
        source_line: Option<String>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            rule_id: rule.rule_id.to_string(),
            pack_id: rule.pack_id.to_string(),
            severity: rule.severity,
            title: rule.title.to_string(),
            summary: summary.into(),
            location,
            requirement: rule.requirement.to_string(),
            source_line,
            label: label.into(),
            labels: rule
                .labels
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect(),
        }
    }
}

/// Public summary for one parsed Rust source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustModuleReport {
    /// Source path.
    pub path: PathBuf,
    /// Whether `syn` parsed the file successfully.
    pub is_valid: bool,
    /// Syntax error when parsing failed.
    pub parse_error: Option<String>,
}

/// Conventional project paths scanned by the harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustProjectHarnessScope {
    /// Project root.
    pub project_root: PathBuf,
    /// Source roots such as `src`.
    pub source_paths: Vec<PathBuf>,
    /// Test roots such as `tests`.
    pub test_paths: Vec<PathBuf>,
    /// Package-level Rust target paths such as `build.rs`, `examples`, and `benches`.
    #[serde(default)]
    pub package_paths: Vec<PathBuf>,
    /// Fallback roots used when no conventional roots exist.
    pub fallback_paths: Vec<PathBuf>,
}

impl RustProjectHarnessScope {
    /// Return the concrete roots scanned by the parser.
    #[must_use]
    pub fn monitored_paths(&self) -> Vec<PathBuf> {
        let mut selected = Vec::new();
        selected.extend(self.source_paths.iter().cloned());
        selected.extend(self.test_paths.iter().cloned());
        selected.extend(self.package_paths.iter().cloned());
        if selected.is_empty() {
            return self.fallback_paths.clone();
        }
        selected
    }
}

/// Configuration for a Rust project harness run.
///
/// The default configuration covers Rust files under conventional `src/`,
/// `tests/`, `examples/`, and `benches/` roots, plus package entrypoint files
/// such as `build.rs`, for package-level harness runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustHarnessConfig {
    /// Directory names skipped during file discovery.
    pub ignored_dir_names: BTreeSet<String>,
    /// Severities that block assertions.
    pub blocking_severities: BTreeSet<RustDiagnosticSeverity>,
    /// Whether project runs include conventional test roots.
    pub include_tests: bool,
    /// Source directory names, relative to the project root.
    pub source_dir_names: Vec<String>,
    /// Test directory names, relative to the project root.
    pub test_dir_names: Vec<String>,
}

impl Default for RustHarnessConfig {
    fn default() -> Self {
        Self {
            ignored_dir_names: crate::discovery::DEFAULT_IGNORED_DIR_NAMES
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
            blocking_severities: BTreeSet::from([
                RustDiagnosticSeverity::Warning,
                RustDiagnosticSeverity::Error,
            ]),
            include_tests: true,
            source_dir_names: vec!["src".to_string()],
            test_dir_names: vec!["tests".to_string()],
        }
    }
}

/// Aggregated harness report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustHarnessReport {
    /// Parsed source modules.
    pub modules: Vec<RustModuleReport>,
    /// All findings, including advisory findings.
    pub findings: Vec<RustHarnessFinding>,
    /// Roots requested by the caller.
    pub root_paths: Vec<PathBuf>,
    /// Severities that block assertions.
    pub blocking_severities: BTreeSet<RustDiagnosticSeverity>,
    /// Project scope, when the project runner was used.
    pub project_scope: Option<RustProjectHarnessScope>,
}

impl RustHarnessReport {
    /// Number of parsed-valid files.
    #[must_use]
    pub fn parsed_count(&self) -> usize {
        self.modules.iter().filter(|module| module.is_valid).count()
    }

    /// Number of discovered Rust files.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.modules.len()
    }

    /// Return whether there are no configured-blocking findings.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.blocking_findings(None).is_empty()
    }

    /// Return blocking findings for the selected severities.
    #[must_use]
    pub fn blocking_findings(
        &self,
        severities: Option<&BTreeSet<RustDiagnosticSeverity>>,
    ) -> Vec<&RustHarnessFinding> {
        let selected = severities.unwrap_or(&self.blocking_severities);
        self.findings
            .iter()
            .filter(|finding| selected.contains(&finding.severity))
            .collect()
    }

    /// Return non-blocking advisory findings.
    #[must_use]
    pub fn advisory_findings(&self) -> Vec<&RustHarnessFinding> {
        self.findings
            .iter()
            .filter(|finding| finding.severity == RustDiagnosticSeverity::Info)
            .collect()
    }

    /// Assert that the report has no configured-blocking findings.
    ///
    /// # Panics
    ///
    /// Panics with the compact rendered report when blocking findings exist.
    #[track_caller]
    pub fn assert_clean(&self) {
        assert!(
            self.is_clean(),
            "{}",
            crate::render_rust_project_harness(self)
        );
    }
}
