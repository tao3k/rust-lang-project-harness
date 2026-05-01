//! Rust modularity rule pack.

mod catalog;
mod entrypoints;
mod reasoning_tree;
mod source_shape;
mod support;

use crate::parser::ParsedRustModule;
use crate::{RustHarnessFinding, RustHarnessRule, RustProjectHarnessScope};

use super::is_under_any_dir;
use catalog::rules_by_id;
use entrypoints::{
    binary_entrypoint_findings, build_script_entrypoint_findings, crate_facade_findings,
    interface_mod_findings, is_package_entrypoint_file,
};
use reasoning_tree::{
    inline_source_module_findings, module_source_shadow_findings, orphan_source_module_findings,
};
use source_shape::{
    deep_relative_import_findings, glob_import_findings, source_file_bloat_findings,
};

pub(crate) const PACK_ID: &str = "rust.modularity";
pub(crate) const RUST_MOD_R001: &str = "RUST-MOD-R001";
pub(crate) const RUST_MOD_R002: &str = "RUST-MOD-R002";
pub(crate) const RUST_MOD_R003: &str = "RUST-MOD-R003";
pub(crate) const RUST_MOD_R004: &str = "RUST-MOD-R004";
pub(crate) const RUST_MOD_R005: &str = "RUST-MOD-R005";
pub(crate) const RUST_MOD_R006: &str = "RUST-MOD-R006";
pub(crate) const RUST_MOD_R007: &str = "RUST-MOD-R007";
pub(crate) const RUST_MOD_R008: &str = "RUST-MOD-R008";
pub(crate) const RUST_MOD_R009: &str = "RUST-MOD-R009";
pub(crate) const RUST_MOD_R010: &str = "RUST-MOD-R010";

pub(crate) const MAX_SOURCE_EFFECTIVE_LINES: usize = 650;
pub(crate) const MIN_SOURCE_PUBLIC_ITEMS: usize = 12;
pub(crate) const MIN_SOURCE_IMPLEMENTATION_ITEMS: usize = 40;

/// Return compact metadata for Rust modularity rules.
#[must_use]
pub fn rust_modularity_rules() -> Vec<RustHarnessRule> {
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
    findings.extend(module_source_shadow_findings(scope, modules, &rules));
    findings.extend(orphan_source_module_findings(scope, modules, &rules));
    for module in modules {
        let is_source_module = is_under_any_dir(&module.report.path, &scope.source_paths);
        let is_package_entrypoint = is_package_entrypoint_file(scope, &module.report.path);
        if !is_source_module && !is_package_entrypoint {
            continue;
        }
        if !module.report.is_valid {
            continue;
        }
        if is_source_module {
            findings.extend(crate_facade_findings(module, &rules));
            findings.extend(binary_entrypoint_findings(scope, module, &rules));
            findings.extend(interface_mod_findings(module, &rules));
            findings.extend(inline_source_module_findings(module, &rules));
            findings.extend(source_file_bloat_findings(module, &rules));
            findings.extend(deep_relative_import_findings(module, &rules));
            findings.extend(glob_import_findings(module, &rules));
        }
        findings.extend(build_script_entrypoint_findings(scope, module, &rules));
    }
    findings
}
