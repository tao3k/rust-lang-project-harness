//! Default rule-pack execution.

use crate::parser::ParsedRustModule;
use crate::{RustHarnessFinding, RustProjectHarnessScope};

use super::{agent_policy, modularity, project_policy, syntax};

pub(crate) fn evaluate_default_rule_packs(
    scope: Option<&RustProjectHarnessScope>,
    modules: &[ParsedRustModule],
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    findings.extend(syntax::evaluate(modules));
    findings.extend(project_policy::evaluate(scope, modules));
    findings.extend(modularity::evaluate(scope, modules));
    findings.extend(agent_policy::evaluate(scope, modules));
    findings
}
