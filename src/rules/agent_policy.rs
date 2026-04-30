//! Agent-oriented Rust policy rules.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use syn::{Attribute, Item, Visibility};

use crate::parser::{ParsedRustModule, file_location, path_line_location, source_line};
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
        let Some(syntax) = &module.syntax else {
            continue;
        };
        findings.extend(module_intent_findings(module, &syntax.items, &rules));
        findings.extend(public_doc_findings(module, &syntax.items, &rules));
        findings.extend(facade_reexport_findings(module, &syntax.items, &rules));
        findings.extend(generic_public_module_findings(
            module,
            &syntax.items,
            &rules,
        ));
        findings.extend(branch_module_intent_findings(module, &syntax.items, &rules));
    }
    findings.extend(repeated_namespace_findings(scope, modules, &rules));
    findings.extend(generic_module_path_findings(scope, &source_modules, &rules));
    findings.extend(public_name_conflict_findings(&source_modules, &rules));
    findings
}

fn module_intent_findings(
    module: &ParsedRustModule,
    items: &[Item],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if has_module_doc(&module.source) || !has_public_surface(items) {
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
    items: &[Item],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[AGENT_R002];
    items
        .iter()
        .filter_map(|item| {
            let public_name = public_named_item(item)?;
            if has_doc_attr(item_attrs(item)) {
                return None;
            }
            let line = item_span_line(item);
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
    module: &ParsedRustModule,
    items: &[Item],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if module
        .report
        .path
        .file_name()
        .and_then(|name| name.to_str())
        != Some("lib.rs")
    {
        return Vec::new();
    }
    let reexport_count = items
        .iter()
        .filter(|item| matches!(item, Item::Use(item_use) if matches!(item_use.vis, Visibility::Public(_))))
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
    items: &[Item],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[AGENT_R006];
    items
        .iter()
        .filter_map(|item| {
            let Item::Mod(item_mod) = item else {
                return None;
            };
            if !is_public(&item_mod.vis) {
                return None;
            }
            let module_name = item_mod.ident.to_string();
            if !is_generic_public_module_name(&module_name) {
                return None;
            }
            let line = item_span_line(item);
            Some(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} exposes generic public module `{module_name}`.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
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
            let components = relative_namespace_components(scope, &module.report.path)?;
            let generic_segment = components
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
    items: &[Item],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if has_module_doc(&module.source) {
        return Vec::new();
    }
    let child_modules = count_external_child_modules(items);
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
        let Some(components) = relative_namespace_components(scope, &module.report.path) else {
            continue;
        };
        let repeated = repeated_segments(&components);
        if repeated.is_empty() {
            continue;
        }
        let branch = offending_branch(&components, &repeated);
        let (_, branch_repeated) = branches
            .entry(branch)
            .or_insert_with(|| (module.report.path.clone(), BTreeSet::new()));
        branch_repeated.extend(repeated);
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
        let Some(syntax) = &module.syntax else {
            continue;
        };
        for item in &syntax.items {
            let Some(name) = public_named_item(item) else {
                continue;
            };
            names
                .entry(name)
                .or_default()
                .push((module, item_span_line(item)));
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

fn has_module_doc(source: &str) -> bool {
    source
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("#!["))
        .is_some_and(|line| line.starts_with("//!") || line.starts_with("/*!"))
}

fn has_public_surface(items: &[Item]) -> bool {
    items.iter().any(|item| public_named_item(item).is_some())
}

fn count_external_child_modules(items: &[Item]) -> usize {
    items
        .iter()
        .filter(|item| matches!(item, Item::Mod(item_mod) if item_mod.content.is_none()))
        .count()
}

fn public_named_item(item: &Item) -> Option<String> {
    match item {
        Item::Const(item) if is_public(&item.vis) => Some(item.ident.to_string()),
        Item::Enum(item) if is_public(&item.vis) => Some(item.ident.to_string()),
        Item::Fn(item) if is_public(&item.vis) => Some(item.sig.ident.to_string()),
        Item::Mod(item) if is_public(&item.vis) => Some(item.ident.to_string()),
        Item::Static(item) if is_public(&item.vis) => Some(item.ident.to_string()),
        Item::Struct(item) if is_public(&item.vis) => Some(item.ident.to_string()),
        Item::Trait(item) if is_public(&item.vis) => Some(item.ident.to_string()),
        Item::TraitAlias(item) if is_public(&item.vis) => Some(item.ident.to_string()),
        Item::Type(item) if is_public(&item.vis) => Some(item.ident.to_string()),
        Item::Union(item) if is_public(&item.vis) => Some(item.ident.to_string()),
        _ => None,
    }
}

fn item_attrs(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::TraitAlias(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Union(item) => &item.attrs,
        _ => &[],
    }
}

fn has_doc_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("doc"))
}

fn item_span_line(item: &Item) -> usize {
    use syn::spanned::Spanned;
    item.span().start().line.max(1)
}

fn is_public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

fn is_generic_public_module_name(name: &str) -> bool {
    is_generic_module_name(name)
}

fn is_generic_module_name(name: &str) -> bool {
    GENERIC_MODULE_NAMES.contains(&name)
}

fn relative_namespace_components(
    scope: &RustProjectHarnessScope,
    path: &Path,
) -> Option<Vec<String>> {
    let relative = path.strip_prefix(&scope.project_root).ok()?;
    let parent = relative.parent()?;
    let mut components = parent
        .iter()
        .map(|component| component.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let file_stem = relative.file_stem()?.to_string_lossy();
    if !matches!(file_stem.as_ref(), "lib" | "main" | "mod") {
        components.push(file_stem.to_string());
    }
    (!components.is_empty()).then_some(components)
}

fn repeated_segments(components: &[String]) -> BTreeSet<String> {
    let mut counts = BTreeMap::new();
    for component in components {
        *counts.entry(component.clone()).or_insert(0usize) += 1;
    }
    counts
        .into_iter()
        .filter_map(|(component, count)| (count > 1).then_some(component))
        .collect()
}

fn offending_branch(components: &[String], repeated: &BTreeSet<String>) -> PathBuf {
    let deepest_index = components
        .iter()
        .enumerate()
        .filter_map(|(index, component)| repeated.contains(component).then_some(index))
        .max()
        .unwrap_or(components.len().saturating_sub(1));
    components
        .iter()
        .take(deepest_index + 1)
        .collect::<PathBuf>()
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
