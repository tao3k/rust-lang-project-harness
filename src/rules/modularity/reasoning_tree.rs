//! Reasoning-tree reachability and branch-shape policies.

use std::collections::BTreeMap;

use crate::parser::{
    ParsedRustModule, RustModuleTreeFacts, file_location, is_special_rust_entrypoint_path,
    path_line_location, source_line,
};
use crate::rules::display_path;
use crate::{RustHarnessFinding, RustHarnessRule};

use super::{RUST_MOD_R007, RUST_MOD_R008, RUST_MOD_R009};

pub(super) fn module_source_shadow_findings(
    module_tree: &RustModuleTreeFacts,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[RUST_MOD_R007];
    module_tree
        .shadowed_module_sources
        .iter()
        .map(|shadow| {
            RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} and {} both define the same Rust module source.",
                    display_path(&shadow.file_form),
                    display_path(&shadow.mod_form)
                ),
                file_location(&shadow.mod_form),
                None,
                "choose one module source layout for this owner",
            )
        })
        .collect()
}

pub(super) fn orphan_source_module_findings(
    module_tree: &RustModuleTreeFacts,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[RUST_MOD_R009];
    module_tree
        .unreachable_source_files
        .iter()
        .map(|path| {
            RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} is not reachable from a crate or binary module tree.",
                    display_path(path)
                ),
                file_location(path),
                None,
                "attach this file with a parent mod declaration or remove it",
            )
        })
        .collect()
}

pub(super) fn inline_source_module_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if is_special_rust_entrypoint_path(&module.report.path) {
        return Vec::new();
    }
    let rule = &rules[RUST_MOD_R008];
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter_map(|item| item.module.as_ref())
        .filter(|item_mod| item_mod.is_inline && !item_mod.is_cfg_test)
        .map(|item_mod| {
            RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} contains inline module `{}`.",
                    display_path(&module.report.path),
                    item_mod.ident
                ),
                path_line_location(&module.report.path, item_mod.line),
                source_line(&module.source, item_mod.line),
                "move this inline module into its own source file",
            )
        })
        .collect()
}
