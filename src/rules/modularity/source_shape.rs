//! Source file shape and owner-boundary policies.

use std::collections::BTreeMap;

use crate::parser::{ParsedRustModule, file_location, path_line_location, source_line};
use crate::rules::display_path;
use crate::{RustHarnessFinding, RustHarnessRule};

use super::{
    MAX_SOURCE_EFFECTIVE_LINES, MIN_SOURCE_IMPLEMENTATION_ITEMS, MIN_SOURCE_PUBLIC_ITEMS,
    RUST_MOD_R002, RUST_MOD_R003, RUST_MOD_R010,
};

pub(super) fn source_file_bloat_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let effective_lines = module.source_metrics.effective_code_lines;
    if effective_lines < MAX_SOURCE_EFFECTIVE_LINES {
        return Vec::new();
    }
    let public_items = module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| item.is_public)
        .count();
    let implementation_items = module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| item.is_implementation_item)
        .count();
    if public_items < MIN_SOURCE_PUBLIC_ITEMS
        && implementation_items < MIN_SOURCE_IMPLEMENTATION_ITEMS
    {
        return Vec::new();
    }
    let rule = &rules[RUST_MOD_R002];
    vec![RustHarnessFinding::from_rule(
        rule,
        format!(
            "{} carries {effective_lines} effective lines, {public_items} public items, and {implementation_items} top-level implementation items.",
            display_path(&module.report.path)
        ),
        file_location(&module.report.path),
        None,
        "split this source file by responsibility",
    )]
}

pub(super) fn deep_relative_import_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[RUST_MOD_R003];
    module
        .syntax_facts
        .use_statements
        .iter()
        .filter_map(|use_syntax| {
            if use_syntax.contains_deep_relative_import {
                Some(RustHarnessFinding::from_rule(
                    rule,
                    format!(
                        "{} uses deep relative import `super::super`.",
                        display_path(&module.report.path)
                    ),
                    path_line_location(&module.report.path, use_syntax.line),
                    source_line(&module.source, use_syntax.line),
                    "replace deep relative import with a clearer owner boundary",
                ))
            } else {
                None
            }
        })
        .collect()
}

pub(super) fn glob_import_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[RUST_MOD_R010];
    module
        .syntax_facts
        .use_statements
        .iter()
        .filter_map(|use_syntax| {
            if use_syntax.contains_glob_import {
                Some(RustHarnessFinding::from_rule(
                    rule,
                    format!(
                        "{} uses a Rust glob import.",
                        display_path(&module.report.path)
                    ),
                    path_line_location(&module.report.path, use_syntax.line),
                    source_line(&module.source, use_syntax.line),
                    "replace glob import with explicit owner imports",
                ))
            } else {
                None
            }
        })
        .collect()
}
