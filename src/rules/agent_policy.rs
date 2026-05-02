//! Agent-oriented Rust policy rules.

mod dependency_graph;
mod source_surface;

use std::collections::BTreeMap;

use crate::parser::{ParsedRustModule, rust_reasoning_tree_facts};
use crate::{RustDiagnosticSeverity, RustHarnessFinding, RustHarnessRule, RustProjectHarnessScope};

use super::labels;

const PACK_ID: &str = "rust.agent_policy";
pub(super) const AGENT_R001: &str = "AGENT-R001";
pub(super) const AGENT_R002: &str = "AGENT-R002";
pub(super) const AGENT_R003: &str = "AGENT-R003";
pub(super) const AGENT_R004: &str = "AGENT-R004";
pub(super) const AGENT_R005: &str = "AGENT-R005";
pub(super) const AGENT_R006: &str = "AGENT-R006";
pub(super) const AGENT_R007: &str = "AGENT-R007";
pub(super) const AGENT_R008: &str = "AGENT-R008";
pub(super) const AGENT_R009: &str = "AGENT-R009";
pub(super) const AGENT_R010: &str = "AGENT-R010";
pub(super) const AGENT_R011: &str = "AGENT-R011";
pub(super) const AGENT_R012: &str = "AGENT-R012";
pub(super) const AGENT_R013: &str = "AGENT-R013";
pub(super) const AGENT_R014: &str = "AGENT-R014";

/// Return compact metadata for agent-oriented Rust policy rules.
#[must_use]
pub fn rust_agent_policy_rules() -> Vec<RustHarnessRule> {
    rules_by_id().into_values().collect()
}

pub(crate) fn evaluate(
    scope: Option<&RustProjectHarnessScope>,
    modules: &[ParsedRustModule],
) -> Vec<RustHarnessFinding> {
    let Some(scope) = scope else {
        return Vec::new();
    };
    let rules = rules_by_id();
    let mut findings = Vec::new();
    let reasoning_tree = rust_reasoning_tree_facts(scope, modules);
    let module_by_path = modules
        .iter()
        .map(|module| (module.report.path.clone(), module))
        .collect::<BTreeMap<_, _>>();
    let source_modules = modules
        .iter()
        .filter(|module| {
            reasoning_tree
                .module(&module.report.path)
                .is_some_and(|module_facts| module_facts.is_source_module)
        })
        .collect::<Vec<_>>();
    for module in &source_modules {
        if !module.report.is_valid {
            continue;
        };
        findings.extend(source_surface::source_module_findings(
            &reasoning_tree,
            module,
            &rules,
        ));
    }
    findings.extend(source_surface::repeated_namespace_findings(
        &reasoning_tree,
        modules,
        &rules,
    ));
    findings.extend(source_surface::generic_module_path_findings(
        &reasoning_tree,
        &source_modules,
        &rules,
    ));
    findings.extend(source_surface::public_name_conflict_findings(
        &source_modules,
        &rules,
    ));
    findings.extend(source_surface::test_support_reexport_findings(
        &reasoning_tree,
        modules,
        &rules,
    ));
    findings.extend(dependency_graph::dependency_graph_findings(
        &reasoning_tree,
        &module_by_path,
        &rules,
    ));
    findings
}

fn rules_by_id() -> BTreeMap<&'static str, RustHarnessRule> {
    [
        RustHarnessRule::new(
            AGENT_R001,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public module lacks an intent doc",
            "Add a concise `//!` module-level intent doc using `clippy::doc_markdown` style, with technical identifiers in backticks.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R002,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public item lacks a doc comment",
            "Document public Rust boundaries using `clippy::doc_markdown` style so agents can reason from native syntax without guessing intent.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R003,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Namespace path repeats a segment",
            "Keep Rust module namespaces branch-unique, including file stems; rename repeated path segments so agents see one clear ownership path.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R004,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public item name conflicts across namespaces",
            "Give project-level public items unambiguous names or move them behind a clear domain namespace.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R005,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Facade exports too many public groups",
            "Keep facade exports grouped by owner so agents can identify the right repair surface quickly.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R006,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public module name is generic",
            "Name public Rust modules after the domain they own; avoid generic buckets such as utils, common, helpers, or shared.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R007,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Module path segment is generic",
            "Avoid generic Rust module file or directory names in source roots; name paths after the owner responsibility.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R008,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Branch module lacks reasoning-tree intent doc",
            "Document source modules that own multiple resolved child edges with a `//!` intent doc in `clippy::doc_markdown` style.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R009,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Owner dependency cycle crosses branches",
            "Keep owner dependency edges acyclic so agents can follow the reasoning tree without circular repair ownership.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R010,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Owner branch imports another owner leaf",
            "Depend on another owner through its branch boundary instead of importing leaf implementation modules directly.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R011,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Owner fan-out lacks intent doc",
            "Document branch modules that coordinate several owner dependencies with a `//!` intent doc in `clippy::doc_markdown` style.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R012,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public semantic identifier uses a primitive type",
            "Give public semantic identifiers a named domain type so agents can preserve invariants without guessing from parameter names.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R013,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public error boundary uses an application error type",
            "Keep public library error boundaries typed so agents and callers can handle recovery without inspecting application error context.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R014,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Test support re-export is unused",
            "Keep test support facades narrow; re-export only names consumed through the same support surface or used by support helpers.",
            labels("agent-policy"),
        ),
    ]
    .into_iter()
    .map(|rule| (rule.rule_id, rule))
    .collect()
}
