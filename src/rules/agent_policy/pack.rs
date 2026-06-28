//! Agent-oriented Rust policy rule catalog and evaluator.

use std::collections::BTreeMap;

use crate::parser::{ParsedRustModule, rust_reasoning_tree_facts};
use crate::{RustDiagnosticSeverity, RustHarnessFinding, RustHarnessRule, RustProjectHarnessScope};

use super::{algorithm_shape, api_shape, data_shape, dependency_graph, source_surface};
use crate::rules::labels;

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
pub(super) const AGENT_R015: &str = "AGENT-R015";
pub(super) const AGENT_R016: &str = "AGENT-R016";
pub(super) const AGENT_R017: &str = "AGENT-R017";
pub(super) const AGENT_R018: &str = "AGENT-R018";
pub(super) const AGENT_R019: &str = "AGENT-R019";
pub(super) const AGENT_R020: &str = "AGENT-R020";
pub(super) const AGENT_R021: &str = "AGENT-R021";
pub(super) const AGENT_R022: &str = "AGENT-R022";
pub(super) const AGENT_R023: &str = "AGENT-R023";
pub(super) const AGENT_R024: &str = "AGENT-R024";
pub(super) const AGENT_R025: &str = "AGENT-R025";
pub(super) const AGENT_R026: &str = "AGENT-R026";
pub(super) const AGENT_R027: &str = "AGENT-R027";
pub(super) const AGENT_R028: &str = "AGENT-R028";
pub(super) const AGENT_R029: &str = "AGENT-R029";
pub(super) const AGENT_R030: &str = "AGENT-R030";
pub(super) const AGENT_R031: &str = "AGENT-R031";
pub(super) const AGENT_R032: &str = "AGENT-R032";
pub(super) const AGENT_R033: &str = "AGENT-R033";
pub(super) const AGENT_R034: &str = "AGENT-R034";

/// Scenario coverage required for an agent policy rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RustAgentPolicyScenarioRequirement {
    /// Agent policy rule id that must stay scenario-backed.
    pub rule_id: &'static str,
    /// Stable scenario id expected in `scenario.toml`.
    pub scenario_id: &'static str,
    /// Policy id expected in `scenario.toml` `policy_ids`.
    pub policy_id: &'static str,
    /// Crate-relative scenario root.
    pub scenario_root: &'static str,
}

const AGENT_POLICY_SCENARIO_REQUIREMENTS: &[RustAgentPolicyScenarioRequirement] = &[
    agent_policy_scenario_requirement(
        AGENT_R015,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1",
    ),
    agent_policy_scenario_requirement(
        AGENT_R016,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1",
    ),
    agent_policy_scenario_requirement(
        AGENT_R017,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1",
    ),
    agent_policy_scenario_requirement(
        AGENT_R025,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1",
    ),
    agent_policy_scenario_requirement(
        AGENT_R026,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1",
    ),
    agent_policy_scenario_requirement(
        AGENT_R029,
        "data-structure-linear-membership-scan-v1",
        "RUST-AGENT-DS-001",
        "tests/unit/scenarios/software_criteria/data_structure_linear_membership_scan_v1",
    ),
    agent_policy_scenario_requirement(
        AGENT_R030,
        "async-blocking-boundary-v1",
        "RUST-AGENT-ASYNC-BLOCKING-001",
        "tests/unit/scenarios/software_criteria/async_blocking_boundary_v1",
    ),
    agent_policy_scenario_requirement(
        AGENT_R031,
        "async-sync-lock-boundary-v1",
        "RUST-AGENT-ASYNC-SYNC-LOCK-001",
        "tests/unit/scenarios/software_criteria/async_sync_lock_boundary_v1",
    ),
    agent_policy_scenario_requirement(
        AGENT_R032,
        "async-backpressure-boundary-v1",
        "RUST-AGENT-ASYNC-BACKPRESSURE-001",
        "tests/unit/scenarios/software_criteria/async_backpressure_boundary_v1",
    ),
    agent_policy_scenario_requirement(
        AGENT_R033,
        "async-select-cancellation-safety-v1",
        "RUST-AGENT-ASYNC-CANCEL-SAFETY-001",
        "tests/unit/scenarios/software_criteria/async_select_cancellation_safety_v1",
    ),
    agent_policy_scenario_requirement(
        AGENT_R034,
        "async-timeout-cancellation-safety-v1",
        "RUST-AGENT-ASYNC-CANCEL-SAFETY-002",
        "tests/unit/scenarios/software_criteria/async_timeout_cancellation_safety_v1",
    ),
];

const fn agent_policy_scenario_requirement(
    rule_id: &'static str,
    scenario_id: &'static str,
    policy_id: &'static str,
    scenario_root: &'static str,
) -> RustAgentPolicyScenarioRequirement {
    RustAgentPolicyScenarioRequirement {
        rule_id,
        scenario_id,
        policy_id,
        scenario_root,
    }
}

/// Return compact metadata for agent-oriented Rust policy rules.
#[must_use]
pub fn rust_agent_policy_rules() -> Vec<RustHarnessRule> {
    rules_by_id().into_values().collect()
}

/// Return the agent policy rules that require scenario benchmark coverage.
#[must_use]
pub(crate) fn rust_agent_policy_scenario_requirements()
-> &'static [RustAgentPolicyScenarioRequirement] {
    AGENT_POLICY_SCENARIO_REQUIREMENTS
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
        findings.extend(data_shape::data_shape_findings(module, &rules));
        findings.extend(api_shape::api_shape_findings(module, &rules));
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
    findings.extend(algorithm_shape::algorithm_shape_findings(
        &source_modules,
        &rules,
    ));
    findings.extend(algorithm_shape::native_iterator_idiom_findings(
        &source_modules,
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
        RustHarnessRule::new(
            AGENT_R015,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public function hides algorithm behind nested control flow",
            "Expose public Rust algorithm shape through guard clauses, `match`, typed dispatch, or small named pipeline steps so agents can reason about the branch structure before editing.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R016,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public function owns a broad linear algorithm surface",
            "Split broad public Rust functions into small named helpers or pipeline steps so agents can edit one algorithm responsibility at a time.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R017,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public function manually spells an iterator transform",
            "Use Rust iterator adapters and consumers such as `map`, `filter`, `filter_map`, `collect`, `sum`, `count`, `any`, `all`, or a named iterator pipeline helper when loops only map, filter, collect, count, sum, answer a predicate, or repeatedly scan the same iterator source.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R018,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public function exposes multiple flag parameters",
            "Replace multiple public `bool` or `Option<bool>` parameters with a named enum, newtype, or config struct so agents can preserve mode semantics without reading every branch.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R019,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public function exposes a broad positional parameter surface",
            "Replace broad public positional parameter lists with a named config, request type, or builder surface so agents can preserve constructor semantics without re-reading every call site.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R020,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public data struct exposes primitive semantic fields",
            "Wrap repeated public semantic primitive fields in named domain types so agents preserve data invariants instead of extending stringly typed state.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R021,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public enum variant exposes primitive semantic payload fields",
            "Move broad public enum variant payloads into named domain types or a named payload struct so agents preserve event and command invariants instead of extending raw primitive state.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R022,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public generic data type carries duplicated derivable bounds",
            "Move derivable or formatting trait bounds off public data type definitions and onto the impl or methods that need them so agents do not over-constrain the API contract.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R023,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public API exposes an anonymous primitive tuple",
            "Replace public tuple parameter or return bundles of primitive semantic values with named structs, enums, or newtypes so agents can preserve field intent.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R024,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public enum tuple variant exposes anonymous primitive payload",
            "Replace public enum tuple variant payloads that bundle primitive semantic values with named fields, named payload structs, or domain newtypes so agents preserve event and command intent.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R025,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Implementation function nests traversal scaffolding",
            "Extract nested internal traversals into named iterator, predicate, or receipt-processing helpers so agents can see the algorithm boundary instead of extending raw loop and guard scaffolding.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R026,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Implementation function manually spells an iterator transform",
            "Use Rust iterator adapters or a named iterator helper when internal loops only map, filter, collect, count, sum, answer a predicate, or repeatedly scan the same iterator source.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R027,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public semantic type alias hides a primitive carrier",
            "Use a public newtype or named struct instead of a primitive type alias for semantic identifiers, tokens, paths, durations, byte sizes, or flags so agents preserve invariants across call sites.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R028,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public data model exposes a stringly state field",
            "Use a public enum, newtype, or typed catalog boundary instead of `String` or `Option<String>` for public state, status, kind, mode, phase, type, tag, or category fields.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R029,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Function performs a linear membership scan inside a loop",
            "Build a `HashSet`, `BTreeSet`, `HashMap`, or `BTreeMap` index before the loop, or document why the nested linear scan is bounded, so agents preserve the algorithmic complexity contract.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R030,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Async task performs blocking work on the runtime",
            "Move blocking I/O, sleeps, or CPU-heavy loops behind `spawn_blocking`, a dedicated worker, or an explicit sync boundary so agents preserve Tokio runtime responsiveness.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R031,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Async task holds a sync lock guard across await",
            "Drop `std::sync` or `parking_lot` lock guards before `.await`, switch the boundary to `tokio::sync`, or isolate the critical section so agents preserve runtime progress and cancellation behavior.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R032,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Async queue lacks a backpressure boundary",
            "Use a bounded channel, expose `poll_ready`/`try_send`/`reserve`, or guard an unbounded channel with an explicit capacity or semaphore boundary so agents preserve async backpressure and memory behavior.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R033,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Tokio select branch uses cancellation-unsafe I/O",
            "Keep `tokio::select!` branches cancellation-safe; avoid `read_exact`, `read_to_end`, `read_to_string`, `write_all`, or `write_all_buf` inside select branches unless the partial-progress contract is explicit.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R034,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Tokio timeout wraps cancellation-unsafe I/O",
            "Keep `tokio::time::timeout` boundaries cancellation-safe; avoid wrapping `read_exact`, `read_to_end`, `read_to_string`, `write_all`, or `write_all_buf` unless partial progress is owned outside the timed future.",
            labels("agent-policy"),
        ),
    ]
    .into_iter()
    .map(|rule| (rule.rule_id, rule))
    .collect()
}
