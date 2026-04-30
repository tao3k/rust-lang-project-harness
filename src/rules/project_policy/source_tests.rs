//! Source-backed test mount policy.

use std::collections::BTreeMap;

use syn::Item;
use syn::spanned::Spanned;

use crate::parser::{ParsedRustModule, path_line_location, source_line};
use crate::rules::{display_path, is_under_any_dir};
use crate::{RustHarnessFinding, RustHarnessRule, RustProjectHarnessScope};

use super::support::{has_cfg_test, path_attr_value, resolve_path_attr};
use super::{RUST_PROJ_R003, RUST_PROJ_R004};

pub(super) fn source_test_mount_findings(
    scope: &RustProjectHarnessScope,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    for module in modules {
        if !is_under_any_dir(&module.report.path, &scope.source_paths) {
            continue;
        }
        let Some(syntax) = &module.syntax else {
            continue;
        };
        collect_source_test_mount_findings(
            &scope.project_root,
            module,
            &syntax.items,
            rules,
            &mut findings,
        );
    }
    findings
}

fn collect_source_test_mount_findings(
    project_root: &std::path::Path,
    module: &ParsedRustModule,
    items: &[Item],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
    findings: &mut Vec<RustHarnessFinding>,
) {
    for item in items {
        let Item::Mod(item_mod) = item else {
            continue;
        };
        if !has_cfg_test(&item_mod.attrs) {
            if let Some((_, nested_items)) = &item_mod.content {
                collect_source_test_mount_findings(
                    project_root,
                    module,
                    nested_items,
                    rules,
                    findings,
                );
            }
            continue;
        }
        let line = item_mod
            .attrs
            .first()
            .map_or(1, |attr| attr.span().start().line);
        if item_mod.content.is_some() {
            let rule = &rules[RUST_PROJ_R003];
            findings.push(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} contains inline #[cfg(test)] module `{}`.",
                    display_path(&module.report.path),
                    item_mod.ident
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "move this test module to tests/unit and mount it with #[path]",
            ));
            continue;
        }
        let Some(path_value) = path_attr_value(&item_mod.attrs) else {
            let rule = &rules[RUST_PROJ_R003];
            findings.push(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} declares cfg(test) module `{}` without an external #[path].",
                    display_path(&module.report.path),
                    item_mod.ident
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "add a #[path = \"...\"] mount into tests/unit",
            ));
            continue;
        };
        let resolved = resolve_path_attr(&module.report.path, &path_value);
        let project_relative = resolved.strip_prefix(project_root).unwrap_or(&resolved);
        if !resolved.exists() || !project_relative.starts_with("tests/unit") {
            let rule = &rules[RUST_PROJ_R004];
            findings.push(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} mounts `{path_value}`, but the resolved path should exist under tests/unit.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "point this external test mount at an existing tests/unit file",
            ));
        }
    }
}
