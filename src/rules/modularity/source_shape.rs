//! Source file shape and owner-boundary policies.

use std::collections::BTreeMap;

use crate::parser::{
    ParsedRustModule, RustReasoningModuleFacts, RustUseGlobScopeKind, RustUseStatementSyntax,
    file_location, path_line_location, source_line,
};
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
            if !use_syntax.deep_relative_imports.is_empty() {
                Some(RustHarnessFinding::from_rule(
                    rule,
                    format!(
                        "{} uses {}.",
                        display_path(&module.report.path),
                        deep_relative_import_descriptor(use_syntax)
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

fn deep_relative_import_descriptor(use_syntax: &RustUseStatementSyntax) -> String {
    let imports = &use_syntax.deep_relative_imports;
    let Some(first_import) = imports.first() else {
        return "deep relative import".to_string();
    };
    if imports.len() > 1 {
        return format!(
            "{} deep relative imports ({})",
            imports.len(),
            imports
                .iter()
                .map(|import| import.rendered_path())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    format!("deep relative import `{}`", first_import.rendered_path())
}

pub(super) fn glob_import_findings(
    module_facts: &RustReasoningModuleFacts,
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
                    glob_import_summary(module_facts, module, use_syntax),
                    path_line_location(&module.report.path, use_syntax.line),
                    source_line(&module.source, use_syntax.line),
                    glob_import_label(use_syntax),
                ))
            } else {
                None
            }
        })
        .collect()
}

fn glob_import_summary(
    module_facts: &RustReasoningModuleFacts,
    module: &ParsedRustModule,
    use_syntax: &RustUseStatementSyntax,
) -> String {
    format!(
        "{} uses {}{}.",
        display_path(&module.report.path),
        glob_import_descriptor(use_syntax),
        glob_import_context(module_facts, use_syntax),
    )
}

fn glob_import_descriptor(use_syntax: &RustUseStatementSyntax) -> String {
    let imports = &use_syntax.glob_imports;
    let Some(first_import) = imports.first() else {
        return "a Rust glob import".to_string();
    };
    if imports.len() > 1 {
        return format!(
            "{} Rust glob imports ({})",
            imports.len(),
            imports
                .iter()
                .map(|glob_import| glob_import.rendered_path())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let rendered_path = first_import.rendered_path();
    if first_import.is_direct_parent_scope_glob {
        return format!("parent-scope glob import `{rendered_path}`");
    }
    if first_import.is_parent_relative_glob {
        return format!("parent-relative glob import `{rendered_path}`");
    }
    if first_import.is_prelude_glob {
        return format!("prelude glob import `{rendered_path}`");
    }
    if first_import.scope_kind == RustUseGlobScopeKind::CrateOwner {
        return format!("crate-owner glob import `{rendered_path}`");
    }
    format!("Rust glob import `{rendered_path}`")
}

fn glob_import_context(
    module_facts: &RustReasoningModuleFacts,
    use_syntax: &RustUseStatementSyntax,
) -> &'static str {
    if module_facts.source_path.is_test_source || use_syntax.context.is_inside_cfg_test_module {
        " in test context"
    } else if use_syntax.context.is_inside_inline_module {
        " inside inline module"
    } else {
        ""
    }
}

fn glob_import_label(use_syntax: &RustUseStatementSyntax) -> &'static str {
    if use_syntax
        .glob_imports
        .iter()
        .any(|glob_import| glob_import.is_direct_parent_scope_glob)
    {
        "replace parent-scope glob with explicit imports"
    } else {
        "replace glob import with explicit owner imports"
    }
}
