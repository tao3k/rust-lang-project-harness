//! Default rule-pack execution.

use crate::parser::ParsedRustModule;
use crate::{RustHarnessConfig, RustHarnessFinding, RustProjectHarnessScope};

use super::{agent_policy, modularity, project_policy, syntax};

pub(crate) fn evaluate_default_rule_packs_with_config(
    scope: Option<&RustProjectHarnessScope>,
    modules: &[ParsedRustModule],
    config: &RustHarnessConfig,
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    findings.extend(syntax::evaluate(modules));
    findings.extend(project_policy::evaluate(scope, modules, config));
    findings.extend(modularity::evaluate(scope, modules));
    findings.extend(agent_policy::evaluate(scope, modules));
    apply_policy_config(findings, config)
}

fn apply_policy_config(
    findings: Vec<RustHarnessFinding>,
    config: &RustHarnessConfig,
) -> Vec<RustHarnessFinding> {
    findings
        .into_iter()
        .filter_map(|mut finding| {
            if config.disabled_rules.contains(&finding.rule_id) {
                return None;
            }
            if let Some(severity) = config.rule_severity_overrides.get(&finding.rule_id) {
                finding.severity = *severity;
            }
            Some(finding)
        })
        .collect()
}
