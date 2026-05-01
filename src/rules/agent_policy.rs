//! Agent-oriented Rust policy rules.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::parser::{
    ParsedRustModule, RustSourcePathFacts, RustTopLevelItemSyntax, file_location,
    path_line_location, rust_source_path_facts, source_line,
};
use crate::{RustDiagnosticSeverity, RustHarnessFinding, RustHarnessRule, RustProjectHarnessScope};

use super::{display_path, is_under_any_dir, labels};

const PACK_ID: &str = "rust.agent_policy";
const AGENT_R001: &str = "AGENT-R001";
const AGENT_R002: &str = "AGENT-R002";
const AGENT_R003: &str = "AGENT-R003";
const AGENT_R004: &str = "AGENT-R004";
const AGENT_R005: &str = "AGENT-R005";
const AGENT_R006: &str = "AGENT-R006";
const AGENT_R007: &str = "AGENT-R007";
const AGENT_R008: &str = "AGENT-R008";

const MAX_FACADE_REEXPORTS: usize = 28;
const MIN_BRANCH_CHILD_MODULES: usize = 2;
const GENERIC_MODULE_NAMES: &[&str] = &[
    "common", "helper", "helpers", "misc", "shared", "stuff", "util", "utils",
];

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
    let source_modules = modules
        .iter()
        .filter(|module| is_under_any_dir(&module.report.path, &scope.source_paths))
        .collect::<Vec<_>>();
    for module in &source_modules {
        if !module.report.is_valid {
            continue;
        };
        findings.extend(module_intent_findings(module, &rules));
        findings.extend(public_doc_findings(module, &rules));
        findings.extend(facade_reexport_findings(scope, module, &rules));
        findings.extend(generic_public_module_findings(module, &rules));
        findings.extend(branch_module_intent_findings(module, &rules));
    }
    findings.extend(repeated_namespace_findings(scope, modules, &rules));
    findings.extend(generic_module_path_findings(scope, &source_modules, &rules));
    findings.extend(public_name_conflict_findings(&source_modules, &rules));
    findings
}

fn module_intent_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if module.syntax_facts.has_module_doc || !has_public_surface(module) {
        return Vec::new();
    }
    let rule = &rules[AGENT_R001];
    vec![RustHarnessFinding::from_rule(
        rule,
        format!(
            "{} has public Rust surface without a module intent doc.",
            display_path(&module.report.path)
        ),
        file_location(&module.report.path),
        source_line(&module.source, 1),
        "add a concise //! module responsibility doc",
    )]
}

fn public_doc_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[AGENT_R002];
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter_map(|item| {
            let public_name = public_named_item(item)?;
            if item.has_doc {
                return None;
            }
            let line = item.line;
            Some(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} exposes public item `{public_name}` without a doc comment.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "add a short doc comment naming the public contract",
            ))
        })
        .collect()
}

fn facade_reexport_findings(
    scope: &RustProjectHarnessScope,
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let path_facts = module_source_path_facts(scope, &module.report.path);
    if !path_facts.is_crate_facade {
        return Vec::new();
    }
    let reexport_count = module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| item.is_public_use)
        .count();
    if reexport_count <= MAX_FACADE_REEXPORTS {
        return Vec::new();
    }
    let rule = &rules[AGENT_R005];
    vec![RustHarnessFinding::from_rule(
        rule,
        format!(
            "{} re-exports {reexport_count} public use groups from the crate facade.",
            display_path(&module.report.path)
        ),
        file_location(&module.report.path),
        source_line(&module.source, 1),
        "group facade exports behind smaller owner modules",
    )]
}

fn generic_public_module_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[AGENT_R006];
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter_map(|item| {
            let module_decl = item.module.as_ref()?;
            if !item.is_public {
                return None;
            }
            let module_name = &module_decl.ident;
            if !is_generic_public_module_name(module_name.as_str()) {
                return None;
            }
            Some(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} exposes generic public module `{module_name}`.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, item.line),
                source_line(&module.source, item.line),
                "rename this public module to the domain it owns",
            ))
        })
        .collect()
}

fn generic_module_path_findings(
    scope: &RustProjectHarnessScope,
    modules: &[&ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[AGENT_R007];
    modules
        .iter()
        .filter_map(|module| {
            let path_facts = module_source_path_facts(scope, &module.report.path);
            let generic_segment = path_facts
                .namespace_components
                .iter()
                .find(|component| is_generic_module_name(component.as_str()))?;
            Some(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} uses generic module path segment `{generic_segment}`.",
                    display_path(&module.report.path)
                ),
                file_location(&module.report.path),
                source_line(&module.source, 1),
                "rename this module path segment to the responsibility it owns",
            ))
        })
        .collect()
}

fn branch_module_intent_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if module.syntax_facts.has_module_doc {
        return Vec::new();
    }
    let child_modules = count_external_child_modules(module);
    if child_modules < MIN_BRANCH_CHILD_MODULES {
        return Vec::new();
    }
    let rule = &rules[AGENT_R008];
    vec![RustHarnessFinding::from_rule(
        rule,
        format!(
            "{} declares {child_modules} child modules without a reasoning-tree intent doc.",
            display_path(&module.report.path)
        ),
        file_location(&module.report.path),
        source_line(&module.source, 1),
        "add a //! doc explaining this branch module's ownership",
    )]
}

fn repeated_namespace_findings(
    scope: &RustProjectHarnessScope,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[AGENT_R003];
    let mut branches = BTreeMap::<PathBuf, (PathBuf, BTreeSet<String>)>::new();
    for module in modules {
        let path_facts = module_source_path_facts(scope, &module.report.path);
        let Some(branch) = path_facts.repeated_namespace_branch else {
            continue;
        };
        if path_facts.repeated_namespace_segments.is_empty() {
            continue;
        }
        let (_, branch_repeated) = branches
            .entry(branch)
            .or_insert_with(|| (module.report.path.clone(), BTreeSet::new()));
        branch_repeated.extend(path_facts.repeated_namespace_segments);
    }
    branches
        .into_iter()
        .map(|(branch, (path, repeated))| {
            let repeated = repeated.into_iter().collect::<Vec<_>>().join(", ");
            RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} repeats namespace segment(s): {repeated}.",
                    display_path(&branch)
                ),
                file_location(path),
                None,
                "rename the deepest repeated path segment to its real responsibility",
            )
        })
        .collect()
}

fn public_name_conflict_findings(
    modules: &[&ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut names = BTreeMap::<String, Vec<(&ParsedRustModule, usize)>>::new();
    for module in modules {
        if !module.report.is_valid {
            continue;
        };
        for item in &module.syntax_facts.top_level_items {
            let Some(name) = public_named_item(item) else {
                continue;
            };
            names
                .entry(name.to_owned())
                .or_default()
                .push((module, item.line));
        }
    }
    let rule = &rules[AGENT_R004];
    let mut findings = Vec::new();
    for (name, locations) in names {
        if locations.len() < 2 {
            continue;
        }
        for (module, line) in locations {
            findings.push(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "Public item `{name}` appears in multiple Rust modules and may be ambiguous for agents."
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "rename or namespace this public item so the owner is unambiguous",
            ));
        }
    }
    findings
}

fn has_public_surface(module: &ParsedRustModule) -> bool {
    module
        .syntax_facts
        .top_level_items
        .iter()
        .any(|item| public_named_item(item).is_some())
}

fn count_external_child_modules(module: &ParsedRustModule) -> usize {
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter_map(|item| item.module.as_ref())
        .filter(|module_decl| !module_decl.is_inline)
        .count()
}

fn public_named_item(item: &RustTopLevelItemSyntax) -> Option<&str> {
    item.is_public.then_some(item.name.as_deref()).flatten()
}

fn is_generic_public_module_name(name: &str) -> bool {
    is_generic_module_name(name)
}

fn is_generic_module_name(name: &str) -> bool {
    GENERIC_MODULE_NAMES.contains(&name)
}

fn module_source_path_facts(
    scope: &RustProjectHarnessScope,
    path: &std::path::Path,
) -> RustSourcePathFacts {
    rust_source_path_facts(
        &scope.project_root,
        &scope.source_paths,
        &scope.package_paths,
        path,
    )
}

fn rules_by_id() -> BTreeMap<&'static str, RustHarnessRule> {
    [
        RustHarnessRule::new(
            AGENT_R001,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public module lacks an intent doc",
            "Add a concise module-level doc comment that names the module responsibility for agent search and repair.",
            labels("agent-policy"),
        ),
        RustHarnessRule::new(
            AGENT_R002,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public item lacks a doc comment",
            "Document public Rust boundaries so agents can reason from native syntax without guessing intent.",
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
            "Document source modules that branch into multiple child modules so agents can traverse the owner tree intentionally.",
            labels("agent-policy"),
        ),
    ]
    .into_iter()
    .map(|rule| (rule.rule_id, rule))
    .collect()
}
