//! Agent policy rules derived from source surface facts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::parser::{
    ParsedRustModule, RustReasoningTreeFacts, RustTopLevelItemSyntax, file_location,
    path_line_location, source_line,
};
use crate::{RustHarnessFinding, RustHarnessRule};

use crate::rules::display_path;

use super::{
    AGENT_R001, AGENT_R002, AGENT_R003, AGENT_R004, AGENT_R005, AGENT_R006, AGENT_R007, AGENT_R008,
};

const MAX_FACADE_REEXPORTS: usize = 28;
const MIN_BRANCH_CHILD_MODULES: usize = 2;
const GENERIC_MODULE_NAMES: &[&str] = &[
    "common", "helper", "helpers", "misc", "shared", "stuff", "util", "utils",
];

pub(super) fn source_module_findings(
    reasoning_tree: &RustReasoningTreeFacts,
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    findings.extend(module_intent_findings(module, rules));
    findings.extend(public_doc_findings(module, rules));
    findings.extend(facade_reexport_findings(reasoning_tree, module, rules));
    findings.extend(generic_public_module_findings(module, rules));
    findings.extend(branch_module_intent_findings(reasoning_tree, module, rules));
    findings
}

pub(super) fn repeated_namespace_findings(
    reasoning_tree: &RustReasoningTreeFacts,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[AGENT_R003];
    let mut branches = BTreeMap::<PathBuf, (PathBuf, BTreeSet<String>)>::new();
    for module in modules {
        let Some(module_facts) = reasoning_tree.module(&module.report.path) else {
            continue;
        };
        let Some(branch) = module_facts.source_path.repeated_namespace_branch.clone() else {
            continue;
        };
        if module_facts
            .source_path
            .repeated_namespace_segments
            .is_empty()
        {
            continue;
        }
        let (_, branch_repeated) = branches
            .entry(branch)
            .or_insert_with(|| (module.report.path.clone(), BTreeSet::new()));
        branch_repeated.extend(module_facts.source_path.repeated_namespace_segments.clone());
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

pub(super) fn generic_module_path_findings(
    reasoning_tree: &RustReasoningTreeFacts,
    modules: &[&ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[AGENT_R007];
    modules
        .iter()
        .filter_map(|module| {
            let module_facts = reasoning_tree.module(&module.report.path)?;
            let generic_segment = module_facts
                .source_path
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

pub(super) fn public_name_conflict_findings(
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
    reasoning_tree: &RustReasoningTreeFacts,
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if !reasoning_tree
        .module(&module.report.path)
        .is_some_and(|module_facts| module_facts.source_path.is_crate_facade)
    {
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
            if !is_generic_module_name(module_name.as_str()) {
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

fn branch_module_intent_findings(
    reasoning_tree: &RustReasoningTreeFacts,
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if module.syntax_facts.has_module_doc {
        return Vec::new();
    }
    let child_modules = reasoning_tree
        .module(&module.report.path)
        .map_or(0, |module_facts| module_facts.declared_child_edges.len());
    if child_modules < MIN_BRANCH_CHILD_MODULES {
        return Vec::new();
    }
    let rule = &rules[AGENT_R008];
    vec![RustHarnessFinding::from_rule(
        rule,
        format!(
            "{} owns {child_modules} resolved child edges without a reasoning-tree intent doc.",
            display_path(&module.report.path)
        ),
        file_location(&module.report.path),
        source_line(&module.source, 1),
        "add a //! doc explaining this branch module's ownership",
    )]
}

fn has_public_surface(module: &ParsedRustModule) -> bool {
    module
        .syntax_facts
        .top_level_items
        .iter()
        .any(|item| public_named_item(item).is_some())
}

fn public_named_item(item: &RustTopLevelItemSyntax) -> Option<&str> {
    item.is_public.then_some(item.name.as_deref()).flatten()
}

fn is_generic_module_name(name: &str) -> bool {
    GENERIC_MODULE_NAMES.contains(&name)
}
