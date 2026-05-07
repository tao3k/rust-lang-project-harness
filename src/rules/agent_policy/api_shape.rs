//! Agent policy rules derived from public API shape facts.

use std::collections::BTreeMap;

use crate::parser::{ParsedRustModule, path_line_location, source_line};
use crate::{RustHarnessFinding, RustHarnessRule};

use crate::rules::display_path;

use super::AGENT_R023;

pub(super) fn api_shape_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    public_tuple_api_surface_findings(module, rules)
}

fn public_tuple_api_surface_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[AGENT_R023];
    module
        .syntax_facts
        .public_tuple_api_surfaces
        .iter()
        .filter(|surface| !surface.is_test_context)
        .map(|surface| {
            let element_list = surface.element_contract_types.join(", ");
            RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} exposes public function `{}` {} as anonymous tuple with primitive elements: {element_list}.",
                    display_path(&module.report.path),
                    surface.function_name,
                    surface.surface_name
                ),
                path_line_location(&module.report.path, surface.line),
                source_line(&module.source, surface.line),
                "replace the tuple with a named struct, enum, or newtype that carries field intent",
            )
        })
        .collect()
}
