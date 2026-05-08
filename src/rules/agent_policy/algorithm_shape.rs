//! Agent policy rules derived from parser-owned function algorithm facts.

use std::collections::BTreeMap;

use crate::parser::{
    ParsedRustModule, RustFunctionControlFlowSyntax, path_line_location, source_line,
};
use crate::{RustHarnessFinding, RustHarnessRule};

use crate::rules::display_path;

use super::{AGENT_R015, AGENT_R016, AGENT_R017, AGENT_R025, AGENT_R026};

const MAX_LINEAR_NESTING_DEPTH: usize = 2;
const MAX_NATIVE_IDIOM_NESTING_DEPTH: usize = 2;
const MIN_BROAD_FUNCTION_LINES: usize = 72;
const MIN_BROAD_FUNCTION_STATEMENTS: usize = 22;
const MIN_LINEAR_BLOCK_STATEMENTS: usize = 14;
const MIN_INTERNAL_TRAVERSAL_NESTING_DEPTH: usize = 4;
const MIN_INTERNAL_TRAVERSAL_LOOP_NESTING_DEPTH: usize = 2;
const MIN_INTERNAL_TRAVERSAL_BRANCH_COUNT: usize = 2;

pub(super) fn algorithm_shape_findings(
    modules: &[&ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    modules
        .iter()
        .flat_map(|module| {
            let mut findings = Vec::new();
            findings.extend(nested_algorithm_findings(module, rules));
            findings.extend(broad_linear_algorithm_findings(module, rules));
            findings.extend(implementation_traversal_findings(module, rules));
            findings.extend(implementation_iterator_idiom_findings(module, rules));
            findings
        })
        .collect()
}

pub(super) fn native_iterator_idiom_findings(
    modules: &[&ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[AGENT_R017];
    modules
        .iter()
        .flat_map(|module| {
            module
                .syntax_facts
                .public_function_control_flows
                .iter()
                .filter(|control_flow| !control_flow.is_test_context)
                .filter_map(|control_flow| {
                    let profile = native_iterator_idiom_profile(control_flow);
                    if profile.is_empty() {
                        return None;
                    }
                    Some(RustHarnessFinding::from_rule(
                        rule,
                        format!(
                            "{} public function `{}` manually spells iterator boilerplate. Signals: {}.",
                            display_path(&module.report.path),
                            control_flow.function_name,
                            profile.join(", ")
                        ),
                        path_line_location(&module.report.path, control_flow.line),
                        source_line(&module.source, control_flow.line),
                        "replace this boilerplate loop with a Rust iterator idiom",
                    ))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn nested_algorithm_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[AGENT_R015];
    module
        .syntax_facts
        .public_function_control_flows
        .iter()
        .filter(|control_flow| !control_flow.is_test_context)
        .filter_map(|control_flow| {
            let profile = nested_algorithm_profile(control_flow);
            if profile.is_empty() {
                return None;
            }
            Some(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} public function `{}` hides algorithm shape. Signals: {}.",
                    display_path(&module.report.path),
                    control_flow.function_name,
                    profile.join(", ")
                ),
                path_line_location(&module.report.path, control_flow.line),
                source_line(&module.source, control_flow.line),
                "make this public algorithm shape explicit",
            ))
        })
        .collect()
}

fn broad_linear_algorithm_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[AGENT_R016];
    module
        .syntax_facts
        .public_function_control_flows
        .iter()
        .filter(|control_flow| !control_flow.is_test_context)
        .filter_map(|control_flow| {
            let profile = broad_linear_profile(control_flow);
            if profile.is_empty() {
                return None;
            }
            Some(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} public function `{}` spans {} lines with {} statements and a {}-statement block. Signals: {}.",
                    display_path(&module.report.path),
                    control_flow.function_name,
                    control_flow.line_span,
                    control_flow.statement_count,
                    control_flow.max_block_statement_count,
                    profile.join(", ")
                ),
                path_line_location(&module.report.path, control_flow.line),
                source_line(&module.source, control_flow.line),
                "split this broad function into named algorithm steps",
            ))
        })
        .collect()
}

fn implementation_traversal_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[AGENT_R025];
    module
        .syntax_facts
        .all_function_control_flows
        .iter()
        .filter(|control_flow| !control_flow.is_public)
        .filter(|control_flow| !control_flow.is_test_context)
        .filter_map(|control_flow| {
            let profile = implementation_traversal_profile(control_flow);
            if profile.is_empty() {
                return None;
            }
            Some(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} implementation function `{}` nests traversal scaffolding. Signals: {}.",
                    display_path(&module.report.path),
                    control_flow.function_name,
                    profile.join(", ")
                ),
                path_line_location(&module.report.path, control_flow.line),
                source_line(&module.source, control_flow.line),
                "extract this traversal into named iterator, predicate, or receipt-processing helpers",
            ))
        })
        .collect()
}

fn implementation_iterator_idiom_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[AGENT_R026];
    module
        .syntax_facts
        .all_function_control_flows
        .iter()
        .filter(|control_flow| !control_flow.is_public)
        .filter(|control_flow| !control_flow.is_test_context)
        .filter(|control_flow| implementation_traversal_profile(control_flow).is_empty())
        .filter_map(|control_flow| {
            let profile = native_iterator_idiom_profile(control_flow);
            if profile.is_empty() {
                return None;
            }
            Some(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} implementation function `{}` manually spells iterator boilerplate. Signals: {}.",
                    display_path(&module.report.path),
                    control_flow.function_name,
                    profile.join(", ")
                ),
                path_line_location(&module.report.path, control_flow.line),
                source_line(&module.source, control_flow.line),
                "extract this loop into Rust iterator adapters or a named helper",
            ))
        })
        .collect()
}

fn nested_algorithm_profile(control_flow: &RustFunctionControlFlowSyntax) -> Vec<&'static str> {
    let mut signals = Vec::new();
    if control_flow.max_nesting_depth >= 4 {
        signals.push("deep control-flow nesting");
    }
    if control_flow.max_loop_nesting_depth >= 2 && control_flow.branch_count >= 3 {
        signals.push("nested loops mixed with branches");
    }
    if control_flow.literal_dispatch_chain_count > 0 {
        signals.push("literal dispatch chain without match");
    }
    if control_flow.branch_count >= 8 && control_flow.match_count == 0 {
        signals.push("large branch surface without match");
    }
    signals
}

fn implementation_traversal_profile(
    control_flow: &RustFunctionControlFlowSyntax,
) -> Vec<&'static str> {
    let mut signals = Vec::new();
    if control_flow.max_nesting_depth >= MIN_INTERNAL_TRAVERSAL_NESTING_DEPTH
        && control_flow.max_loop_nesting_depth >= MIN_INTERNAL_TRAVERSAL_LOOP_NESTING_DEPTH
        && control_flow.branch_count >= MIN_INTERNAL_TRAVERSAL_BRANCH_COUNT
    {
        signals.push("nested loops guarded by branches");
    }
    if control_flow.repeated_iterator_source_loop_count > 0 && control_flow.branch_count >= 2 {
        signals.push("repeated guarded scans over one source");
    }
    signals
}

fn broad_linear_profile(control_flow: &RustFunctionControlFlowSyntax) -> Vec<&'static str> {
    if control_flow.max_nesting_depth > MAX_LINEAR_NESTING_DEPTH {
        return Vec::new();
    }
    let mut signals = Vec::new();
    if control_flow.line_span >= MIN_BROAD_FUNCTION_LINES
        && control_flow.statement_count >= MIN_BROAD_FUNCTION_STATEMENTS
    {
        signals.push("long public function body");
    }
    if control_flow.max_block_statement_count >= MIN_LINEAR_BLOCK_STATEMENTS {
        signals.push("large linear statement block");
    }
    signals
}

fn native_iterator_idiom_profile(
    control_flow: &RustFunctionControlFlowSyntax,
) -> Vec<&'static str> {
    if control_flow.max_nesting_depth > MAX_NATIVE_IDIOM_NESTING_DEPTH {
        return Vec::new();
    }
    let mut signals = Vec::new();
    if control_flow.manual_collection_loop_count > 0 {
        signals.push("manual collection accumulator loop");
    }
    if control_flow.manual_predicate_loop_count > 0 {
        signals.push("manual predicate loop");
    }
    if control_flow.manual_count_loop_count > 0 {
        signals.push("manual count loop");
    }
    if control_flow.manual_numeric_accumulator_loop_count > 0 {
        signals.push("manual numeric accumulator loop");
    }
    if control_flow.repeated_iterator_source_loop_count > 0 {
        signals.push("repeated pass over the same iterator source");
    }
    signals
}
