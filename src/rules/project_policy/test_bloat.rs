//! Test leaf bloat policy.

use std::collections::BTreeMap;
use std::path::Path;

use crate::parser::{file_location, parse_rust_file};
use crate::{RustHarnessFinding, RustHarnessRule};

use super::support::{collect_rust_files, display_project_path};
use super::{
    MAX_INTEGRATION_TEST_EFFECTIVE_LINES, MAX_UNIT_TEST_EFFECTIVE_LINES,
    MIN_INTEGRATION_TEST_FUNCTIONS, MIN_UNIT_TEST_FUNCTIONS, RUST_PROJ_R005,
};

pub(super) fn test_leaf_bloat_findings(
    project_root: &Path,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    collect_leaf_bloat_findings(
        project_root,
        "unit",
        MAX_UNIT_TEST_EFFECTIVE_LINES,
        MIN_UNIT_TEST_FUNCTIONS,
        rules,
        &mut findings,
    );
    collect_leaf_bloat_findings(
        project_root,
        "integration",
        MAX_INTEGRATION_TEST_EFFECTIVE_LINES,
        MIN_INTEGRATION_TEST_FUNCTIONS,
        rules,
        &mut findings,
    );
    findings
}

fn collect_leaf_bloat_findings(
    project_root: &Path,
    suite_name: &str,
    max_effective_lines: usize,
    min_test_functions: usize,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
    findings: &mut Vec<RustHarnessFinding>,
) {
    let suite_dir = project_root.join("tests").join(suite_name);
    let mut files = Vec::new();
    collect_rust_files(&suite_dir, &mut files);
    for path in files {
        let parsed = parse_rust_file(&path);
        let effective_lines = parsed.source_metrics.effective_code_lines;
        if effective_lines < max_effective_lines {
            continue;
        }
        let test_functions = parsed.syntax_facts.test_function_count;
        if test_functions < min_test_functions {
            continue;
        }
        let rule = &rules[RUST_PROJ_R005];
        findings.push(RustHarnessFinding::from_rule(
            rule,
            format!(
                "{} carries {effective_lines} effective lines across {test_functions} test functions.",
                display_project_path(project_root, &path)
            ),
            file_location(path),
            None,
            "split this test leaf into a folder-first suite",
        ));
    }
}
