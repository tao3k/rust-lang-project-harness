//! Shared report model for Rust project harness runs.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::verification::{
    RustVerificationPolicy, RustVerificationProfileHint, RustVerificationReceipt,
    RustVerificationWaiver,
};

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

/// Built-in rule packs that can be configured as a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustRulePack {
    /// Syntax parsing rules.
    Syntax,
    /// Project-level harness and test-layout policy.
    ProjectPolicy,
    /// Rust source modularity and ownership policy.
    Modularity,
    /// Non-blocking repair advice for agents.
    AgentPolicy,
}

impl RustRulePack {
    /// Return the stable rule-pack id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Syntax => "rust.syntax",
            Self::ProjectPolicy => "rust.project_policy",
            Self::Modularity => "rust.modularity",
            Self::AgentPolicy => "rust.agent_policy",
        }
    }

    /// Return all stable rule ids owned by this built-in pack.
    #[must_use]
    pub const fn rule_ids(self) -> &'static [&'static str] {
        match self {
            Self::Syntax => &["RUST-SYN-R001"],
            Self::ProjectPolicy => &[
                "RUST-PROJ-R001",
                "RUST-PROJ-R002",
                "RUST-PROJ-R003",
                "RUST-PROJ-R004",
                "RUST-PROJ-R005",
                "RUST-PROJ-R006",
                "RUST-PROJ-R007",
                "RUST-PROJ-R008",
                "RUST-PROJ-R009",
            ],
            Self::Modularity => &[
                "RUST-MOD-R001",
                "RUST-MOD-R002",
                "RUST-MOD-R003",
                "RUST-MOD-R004",
                "RUST-MOD-R005",
                "RUST-MOD-R006",
                "RUST-MOD-R007",
                "RUST-MOD-R008",
                "RUST-MOD-R009",
                "RUST-MOD-R010",
            ],
            Self::AgentPolicy => &[
                "AGENT-R001",
                "AGENT-R002",
                "AGENT-R003",
                "AGENT-R004",
                "AGENT-R005",
                "AGENT-R006",
                "AGENT-R007",
                "AGENT-R008",
                "AGENT-R009",
                "AGENT-R010",
                "AGENT-R011",
            ],
        }
    }
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
    /// Short display title.
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
    /// Rule ids that should not emit findings for this run.
    #[serde(default)]
    pub disabled_rules: BTreeSet<String>,
    /// Per-rule severity overrides applied after rule evaluation.
    #[serde(default)]
    pub rule_severity_overrides: BTreeMap<String, RustDiagnosticSeverity>,
    /// Whether project runs include conventional test roots.
    pub include_tests: bool,
    /// Source directory names, relative to the project root.
    pub source_dir_names: Vec<String>,
    /// Test directory names, relative to the project root.
    pub test_dir_names: Vec<String>,
    /// Library-first verification policy used to plan external skill tasks.
    #[serde(default, skip_serializing_if = "RustVerificationPolicy::is_empty")]
    pub verification_policy: RustVerificationPolicy,
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
            disabled_rules: BTreeSet::new(),
            rule_severity_overrides: BTreeMap::new(),
            include_tests: true,
            source_dir_names: vec!["src".to_string()],
            test_dir_names: vec!["tests".to_string()],
            verification_policy: RustVerificationPolicy::default(),
        }
    }
}

impl RustHarnessConfig {
    /// Return a config with one rule disabled.
    #[must_use]
    pub fn with_disabled_rule(mut self, rule_id: impl Into<String>) -> Self {
        self.disabled_rules.insert(rule_id.into());
        self
    }

    /// Return a config with several rules disabled.
    #[must_use]
    pub fn with_disabled_rules<I, S>(mut self, rule_ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.disabled_rules
            .extend(rule_ids.into_iter().map(Into::into));
        self
    }

    /// Return a config with every rule in one built-in pack disabled.
    #[must_use]
    pub fn with_disabled_rule_pack(mut self, rule_pack: RustRulePack) -> Self {
        self.disabled_rules.extend(
            rule_pack
                .rule_ids()
                .iter()
                .map(|rule_id| (*rule_id).to_string()),
        );
        self
    }

    /// Return a config with one rule severity overridden.
    #[must_use]
    pub fn with_rule_severity(
        mut self,
        rule_id: impl Into<String>,
        severity: RustDiagnosticSeverity,
    ) -> Self {
        self.rule_severity_overrides
            .insert(rule_id.into(), severity);
        self
    }

    /// Return a config with every rule in one built-in pack assigned a severity.
    #[must_use]
    pub fn with_rule_pack_severity(
        mut self,
        rule_pack: RustRulePack,
        severity: RustDiagnosticSeverity,
    ) -> Self {
        self.rule_severity_overrides.extend(
            rule_pack
                .rule_ids()
                .iter()
                .map(|rule_id| ((*rule_id).to_string(), severity)),
        );
        self
    }

    /// Return a config with explicit blocking severities.
    #[must_use]
    pub fn with_blocking_severities<I>(mut self, severities: I) -> Self
    where
        I: IntoIterator<Item = RustDiagnosticSeverity>,
    {
        self.blocking_severities = severities.into_iter().collect();
        self
    }

    /// Return a config with an explicit verification policy.
    #[must_use]
    pub fn with_verification_policy(mut self, policy: RustVerificationPolicy) -> Self {
        self.verification_policy = policy;
        self
    }

    /// Return a config with one verification profile hint appended.
    #[must_use]
    pub fn with_verification_profile_hint(mut self, hint: RustVerificationProfileHint) -> Self {
        self.verification_policy.profile_hints.push(hint);
        self
    }

    /// Return a config with one verification receipt appended.
    #[must_use]
    pub fn with_verification_receipt(mut self, receipt: RustVerificationReceipt) -> Self {
        self.verification_policy.receipts.push(receipt);
        self
    }

    /// Return a config with one verification waiver appended.
    #[must_use]
    pub fn with_verification_waiver(mut self, waiver: RustVerificationWaiver) -> Self {
        self.verification_policy.waivers.push(waiver);
        self
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
    /// Cargo member scopes, when a workspace or package collection was scanned.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workspace_member_scopes: Vec<RustProjectHarnessScope>,
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
