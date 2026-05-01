use std::collections::BTreeSet;

use rust_lang_project_harness::{
    RustHarnessConfig, RustOwnerResponsibility, RustVerificationProfileHint,
    RustVerificationReceipt, RustVerificationSkillBinding, RustVerificationSkillDescriptor,
    RustVerificationTaskKind, RustVerificationTaskState, RustVerificationWaiver,
    default_rust_harness_config, plan_rust_project_verification_with_config,
    render_rust_verification_plan, render_rust_verification_skill_contracts,
};
use tempfile::TempDir;

use crate::verification::support::{normalize_temp_root, write_api_project};

const SKILL_CONTRACT_SCENARIOS: &[VerificationSkillContractScenario] = &[
    VerificationSkillContractScenario {
        id: "stress_k6_triggered_by_public_api",
        kind: RustVerificationTaskKind::Stress,
        responsibility: Some(RustOwnerResponsibility::PublicApi),
        descriptor: SkillDescriptorCase::K6Stress,
        resolution: ScenarioResolution::None,
    },
    VerificationSkillContractScenario {
        id: "performance_criterion_triggered_by_latency_sensitive",
        kind: RustVerificationTaskKind::Performance,
        responsibility: Some(RustOwnerResponsibility::LatencySensitive),
        descriptor: SkillDescriptorCase::CriterionPerformance,
        resolution: ScenarioResolution::None,
    },
    VerificationSkillContractScenario {
        id: "descriptor_config_without_responsibility_is_not_triggered",
        kind: RustVerificationTaskKind::Performance,
        responsibility: None,
        descriptor: SkillDescriptorCase::CriterionPerformance,
        resolution: ScenarioResolution::None,
    },
    VerificationSkillContractScenario {
        id: "public_api_profile_suppressed_with_rationale_is_not_triggered",
        kind: RustVerificationTaskKind::Stress,
        responsibility: Some(RustOwnerResponsibility::PublicApi),
        descriptor: SkillDescriptorCase::K6Stress,
        resolution: ScenarioResolution::ProfileSuppressed,
    },
    VerificationSkillContractScenario {
        id: "stress_k6_solved_by_passed_receipt",
        kind: RustVerificationTaskKind::Stress,
        responsibility: Some(RustOwnerResponsibility::PublicApi),
        descriptor: SkillDescriptorCase::K6Stress,
        resolution: ScenarioResolution::PassedReceipt,
    },
    VerificationSkillContractScenario {
        id: "performance_criterion_solved_by_complete_waiver",
        kind: RustVerificationTaskKind::Performance,
        responsibility: Some(RustOwnerResponsibility::LatencySensitive),
        descriptor: SkillDescriptorCase::CriterionPerformance,
        resolution: ScenarioResolution::CompleteWaiver,
    },
];

#[derive(Clone, Copy)]
struct VerificationSkillContractScenario {
    id: &'static str,
    kind: RustVerificationTaskKind,
    responsibility: Option<RustOwnerResponsibility>,
    descriptor: SkillDescriptorCase,
    resolution: ScenarioResolution,
}

#[derive(Clone, Copy)]
enum SkillDescriptorCase {
    K6Stress,
    CriterionPerformance,
}

impl SkillDescriptorCase {
    fn descriptor(self) -> RustVerificationSkillDescriptor {
        match self {
            Self::K6Stress => RustVerificationSkillDescriptor::k6_stress(),
            Self::CriterionPerformance => RustVerificationSkillDescriptor::criterion_performance(),
        }
    }

    fn binding(self) -> RustVerificationSkillBinding {
        let descriptor = self.descriptor();
        let mut binding = RustVerificationSkillBinding::new(descriptor.skill_id);
        if let Some(adapter) = descriptor.adapter {
            binding = binding.with_adapter(adapter);
        }
        binding
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScenarioResolution {
    None,
    ProfileSuppressed,
    PassedReceipt,
    CompleteWaiver,
}

#[test]
fn verification_skill_contract_scenario_ids_are_unique() {
    let mut ids = BTreeSet::new();

    for scenario in SKILL_CONTRACT_SCENARIOS {
        assert!(
            ids.insert(scenario.id),
            "duplicate scenario id: {}",
            scenario.id
        );
    }
}

#[test]
fn verification_skill_contract_scenarios_snapshot() {
    let rendered = SKILL_CONTRACT_SCENARIOS
        .iter()
        .map(render_skill_contract_scenario)
        .collect::<Vec<_>>()
        .join("\n");

    insta::assert_snapshot!("verification_skill_contract_scenarios", rendered);
}

fn render_skill_contract_scenario(scenario: &VerificationSkillContractScenario) -> String {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = scenario.initial_config();
    let initial_plan =
        plan_rust_project_verification_with_config(root, &config).expect("initial scenario plan");
    let final_config = match scenario.resolution {
        ScenarioResolution::None | ScenarioResolution::ProfileSuppressed => config,
        ScenarioResolution::PassedReceipt => {
            let task = initial_plan
                .active_tasks()
                .into_iter()
                .find(|task| task.kind == scenario.kind)
                .expect("active task for receipt scenario");
            config.with_verification_receipt(RustVerificationReceipt::passed(
                task.fingerprint.clone(),
                scenario.kind,
            ))
        }
        ScenarioResolution::CompleteWaiver => {
            let task = initial_plan
                .active_tasks()
                .into_iter()
                .find(|task| task.kind == scenario.kind)
                .expect("active task for waiver scenario");
            config.with_verification_waiver(RustVerificationWaiver::new(
                task.fingerprint.clone(),
                "platform",
                "scenario coverage handles this verification slice",
                "2026-06-01",
            ))
        }
    };
    let final_plan = plan_rust_project_verification_with_config(root, &final_config)
        .expect("final scenario plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&final_plan), root);
    let contracts = render_rust_verification_skill_contracts(&final_plan);
    let initial_status = scenario_status(&initial_plan, scenario.kind);
    let final_status = scenario_status(&final_plan, scenario.kind);

    assert_scenario_expectations(
        scenario,
        &initial_status,
        &final_status,
        &rendered,
        &contracts,
    );
    render_scenario_audit(
        scenario,
        &initial_status,
        &final_status,
        &rendered,
        &contracts,
    )
}

impl VerificationSkillContractScenario {
    fn initial_config(self) -> RustHarnessConfig {
        let mut config = default_rust_harness_config()
            .with_verification_skill_binding(self.kind, self.descriptor.binding())
            .with_verification_skill_descriptor(self.descriptor.descriptor());
        if let Some(responsibility) = self.responsibility {
            let mut hint = RustVerificationProfileHint::new("src/api.rs", [responsibility]);
            if self.resolution == ScenarioResolution::ProfileSuppressed {
                hint = hint
                    .without_verification_tasks()
                    .with_rationale("scenario intentionally suppresses external verification");
            }
            config = config.with_verification_profile_hint(hint);
        }
        config
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScenarioStatus {
    active_count: usize,
    task_state: Option<RustVerificationTaskState>,
    contract_ref: Option<String>,
}

fn scenario_status(
    plan: &rust_lang_project_harness::RustVerificationPlan,
    kind: RustVerificationTaskKind,
) -> ScenarioStatus {
    let task = plan.tasks.iter().find(|task| task.kind == kind);
    ScenarioStatus {
        active_count: plan.active_tasks().len(),
        task_state: task.map(|task| task.state),
        contract_ref: task
            .filter(|task| task.is_active())
            .and_then(|task| task.skill_contract_ref.clone()),
    }
}

fn assert_scenario_expectations(
    scenario: &VerificationSkillContractScenario,
    initial_status: &ScenarioStatus,
    final_status: &ScenarioStatus,
    rendered: &str,
    contracts: &str,
) {
    match scenario.resolution {
        ScenarioResolution::None if scenario.responsibility.is_some() => {
            assert_eq!(initial_status.active_count, 1, "{}", scenario.id);
            assert_eq!(final_status.active_count, 1, "{}", scenario.id);
            assert_eq!(
                final_status.task_state,
                Some(RustVerificationTaskState::Pending),
                "{}",
                scenario.id
            );
            assert!(
                rendered.contains("contract_ref="),
                "triggered scenario should keep a compact contract ref: {}",
                scenario.id
            );
            assert!(
                contracts.contains("[skill-contract]"),
                "triggered scenario should expose an expandable contract: {}",
                scenario.id
            );
        }
        ScenarioResolution::None => {
            assert_eq!(initial_status.active_count, 0, "{}", scenario.id);
            assert_eq!(final_status.active_count, 0, "{}", scenario.id);
            assert!(rendered.is_empty(), "{}", scenario.id);
            assert!(contracts.is_empty(), "{}", scenario.id);
        }
        ScenarioResolution::ProfileSuppressed => {
            assert_eq!(initial_status.active_count, 0, "{}", scenario.id);
            assert_eq!(final_status.active_count, 0, "{}", scenario.id);
            assert!(rendered.is_empty(), "{}", scenario.id);
            assert!(contracts.is_empty(), "{}", scenario.id);
        }
        ScenarioResolution::PassedReceipt => {
            assert_eq!(initial_status.active_count, 1, "{}", scenario.id);
            assert_eq!(final_status.active_count, 0, "{}", scenario.id);
            assert_eq!(
                final_status.task_state,
                Some(RustVerificationTaskState::Satisfied),
                "{}",
                scenario.id
            );
            assert!(rendered.is_empty(), "{}", scenario.id);
            assert!(contracts.is_empty(), "{}", scenario.id);
        }
        ScenarioResolution::CompleteWaiver => {
            assert_eq!(initial_status.active_count, 1, "{}", scenario.id);
            assert_eq!(final_status.active_count, 0, "{}", scenario.id);
            assert_eq!(
                final_status.task_state,
                Some(RustVerificationTaskState::Waived),
                "{}",
                scenario.id
            );
            assert!(rendered.is_empty(), "{}", scenario.id);
            assert!(contracts.is_empty(), "{}", scenario.id);
        }
    }
}

fn render_scenario_audit(
    scenario: &VerificationSkillContractScenario,
    initial_status: &ScenarioStatus,
    final_status: &ScenarioStatus,
    rendered: &str,
    contracts: &str,
) -> String {
    let mut audit = format!(
        "[scenario] {}\n   |kind: {}\n   |resolution: {}\n   |initial: active={} state={} contract_ref={}\n   |final: active={} state={} contract_ref={}\n",
        scenario.id,
        scenario.kind.as_str(),
        scenario.resolution.label(),
        initial_status.active_count,
        state_label(initial_status.task_state),
        option_label(initial_status.contract_ref.as_deref()),
        final_status.active_count,
        state_label(final_status.task_state),
        option_label(final_status.contract_ref.as_deref()),
    );
    append_block(&mut audit, "verify", rendered);
    append_block(&mut audit, "contracts", contracts);
    audit
}

impl ScenarioResolution {
    const fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ProfileSuppressed => "profile_suppressed",
            Self::PassedReceipt => "passed_receipt",
            Self::CompleteWaiver => "complete_waiver",
        }
    }
}

fn state_label(state: Option<RustVerificationTaskState>) -> &'static str {
    match state {
        Some(RustVerificationTaskState::Pending) => "pending",
        Some(RustVerificationTaskState::Satisfied) => "satisfied",
        Some(RustVerificationTaskState::Failed) => "failed",
        Some(RustVerificationTaskState::Waived) => "waived",
        None => "-",
    }
}

fn option_label(value: Option<&str>) -> &str {
    value.unwrap_or("-")
}

fn append_block(audit: &mut String, label: &str, block: &str) {
    if block.is_empty() {
        audit.push_str(&format!("   |{label}: -\n"));
        return;
    }
    for line in block.lines() {
        audit.push_str(&format!("   |{label}: {line}\n"));
    }
}
