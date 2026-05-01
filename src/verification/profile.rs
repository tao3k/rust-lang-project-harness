//! Verification profile mapping and default task contracts.

use std::collections::BTreeSet;

use super::{
    RustOwnerResponsibility, RustVerificationEvidence, RustVerificationPhase,
    RustVerificationPolicy, RustVerificationProfileHint, RustVerificationRequirement,
    RustVerificationTaskContract, RustVerificationTaskKind,
};

pub(super) fn responsibility_labels(
    responsibilities: &BTreeSet<RustOwnerResponsibility>,
) -> String {
    if responsibilities.is_empty() {
        return "<none>".to_string();
    }
    responsibilities
        .iter()
        .map(|responsibility| responsibility.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn task_kind_labels(task_kinds: &BTreeSet<RustVerificationTaskKind>) -> String {
    if task_kinds.is_empty() {
        return "<none>".to_string();
    }
    task_kinds
        .iter()
        .map(|task_kind| task_kind.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn profile_evidence(
    hint: &RustVerificationProfileHint,
) -> Vec<RustVerificationEvidence> {
    let mut evidence = vec![RustVerificationEvidence::new(
        "profile",
        responsibility_labels(&hint.responsibilities),
    )];
    if let Some(rationale) = normalized_hint_rationale(hint) {
        evidence.push(RustVerificationEvidence::new("rationale", rationale));
    }
    evidence
}

pub(super) fn hint_rationale_is_empty(hint: &RustVerificationProfileHint) -> bool {
    normalized_hint_rationale(hint).is_none()
}

fn normalized_hint_rationale(hint: &RustVerificationProfileHint) -> Option<String> {
    hint.rationale
        .as_deref()
        .map(str::trim)
        .filter(|rationale| !rationale.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn task_kinds_for_responsibilities(
    responsibilities: &BTreeSet<RustOwnerResponsibility>,
    policy: &RustVerificationPolicy,
) -> BTreeSet<RustVerificationTaskKind> {
    responsibilities
        .iter()
        .flat_map(|responsibility| {
            policy
                .responsibility_task_overrides
                .get(responsibility)
                .cloned()
                .unwrap_or_else(|| default_task_kinds_for_responsibility(*responsibility))
        })
        .collect()
}

pub(super) fn task_kinds_for_profile(
    hint: &RustVerificationProfileHint,
    policy: &RustVerificationPolicy,
) -> BTreeSet<RustVerificationTaskKind> {
    hint.task_kinds
        .clone()
        .unwrap_or_else(|| task_kinds_for_responsibilities(&hint.responsibilities, policy))
}

fn default_task_kinds_for_responsibility(
    responsibility: RustOwnerResponsibility,
) -> BTreeSet<RustVerificationTaskKind> {
    match responsibility {
        RustOwnerResponsibility::PublicApi | RustOwnerResponsibility::LatencySensitive => {
            BTreeSet::from([RustVerificationTaskKind::Stress])
        }
        RustOwnerResponsibility::ExternalDependency
        | RustOwnerResponsibility::Persistence
        | RustOwnerResponsibility::AvailabilityCritical => {
            BTreeSet::from([RustVerificationTaskKind::Chaos])
        }
        RustOwnerResponsibility::SecurityBoundary => {
            BTreeSet::from([RustVerificationTaskKind::Security])
        }
        RustOwnerResponsibility::PureDomainLogic => BTreeSet::new(),
    }
}

pub(super) fn profile_task_reason(
    kind: RustVerificationTaskKind,
    responsibilities: &BTreeSet<RustOwnerResponsibility>,
    uses_owner_task_override: bool,
) -> String {
    if uses_owner_task_override {
        return format!(
            "owner profile explicitly requests {} verification",
            kind.as_str()
        );
    }
    if responsibilities.iter().any(|responsibility| {
        default_task_kinds_for_responsibility(*responsibility).contains(&kind)
    }) {
        default_profile_task_reason(kind).to_string()
    } else {
        format!(
            "profile config maps responsibilities to {} verification",
            kind.as_str()
        )
    }
}

fn default_profile_task_reason(kind: RustVerificationTaskKind) -> &'static str {
    match kind {
        RustVerificationTaskKind::Stress => "profile declares public or latency-sensitive surface",
        RustVerificationTaskKind::Chaos => {
            "profile declares dependency, persistence, or availability responsibility"
        }
        RustVerificationTaskKind::Security => {
            "profile declares auth, authorization, secret, or trust-boundary logic"
        }
        RustVerificationTaskKind::Regression => {
            "profile config maps responsibilities to regression verification"
        }
        RustVerificationTaskKind::ResponsibilityReview => {
            "profile config maps responsibilities to responsibility review"
        }
    }
}

pub(super) fn task_contract_for_profile(
    policy: &RustVerificationPolicy,
    hint: Option<&RustVerificationProfileHint>,
    kind: RustVerificationTaskKind,
) -> RustVerificationTaskContract {
    hint.and_then(|hint| hint.task_contract_overrides.get(&kind).cloned())
        .or_else(|| policy.task_contract_overrides.get(&kind).cloned())
        .unwrap_or_else(|| default_task_contract(kind))
}

fn default_task_contract(kind: RustVerificationTaskKind) -> RustVerificationTaskContract {
    match kind {
        RustVerificationTaskKind::Stress => RustVerificationTaskContract::new(
            RustVerificationPhase::AfterUnitTestsPass,
            "stress skill must report p50/p99/p999, load steps, and SLA result for this fingerprint",
            stress_requirements(),
        ),
        RustVerificationTaskKind::Chaos => RustVerificationTaskContract::new(
            RustVerificationPhase::BeforeRelease,
            "chaos skill must report injected failures, degradation behavior, and recovery result for this fingerprint",
            chaos_requirements(),
        ),
        RustVerificationTaskKind::Security => RustVerificationTaskContract::new(
            RustVerificationPhase::BeforeRelease,
            "security skill must report scanned attack classes and authorization-boundary result for this fingerprint",
            security_requirements(),
        ),
        RustVerificationTaskKind::Regression => RustVerificationTaskContract::new(
            RustVerificationPhase::ScheduledRegression,
            "regression skill must report source growth, dependency drift, and module-cycle status for this fingerprint",
            regression_requirements(),
        ),
        RustVerificationTaskKind::ResponsibilityReview => RustVerificationTaskContract::new(
            RustVerificationPhase::BeforeVerification,
            "update the verification profile hint to match parser facts, or attach a complete waiver",
            responsibility_review_requirements(),
        ),
    }
}

fn stress_requirements() -> Vec<RustVerificationRequirement> {
    requirements([
        ("p50", "median latency under the chosen load step"),
        ("p99", "p99 latency under the chosen load step"),
        (
            "p999",
            "p999 latency when available or explicitly unsupported",
        ),
        (
            "load_steps",
            "pressure staircase and concurrency/request rates",
        ),
        ("sla_result", "whether the declared SLA was held or broken"),
    ])
}

fn chaos_requirements() -> Vec<RustVerificationRequirement> {
    requirements([
        (
            "injected_failures",
            "dependencies and failure modes injected",
        ),
        ("degradation", "observed degraded behavior during the fault"),
        (
            "recovery",
            "recovery signal and time after the fault is removed",
        ),
    ])
}

fn security_requirements() -> Vec<RustVerificationRequirement> {
    requirements([
        ("attack_classes", "common attack classes scanned"),
        (
            "authorization_boundary",
            "authorization or trust-boundary result",
        ),
        ("findings", "confirmed findings or explicit none result"),
    ])
}

fn regression_requirements() -> Vec<RustVerificationRequirement> {
    requirements([
        ("source_growth", "source growth or owner bloat trend"),
        (
            "dependency_drift",
            "owner dependency drift or fan-out change",
        ),
        ("module_cycles", "module or owner-cycle status"),
    ])
}

fn responsibility_review_requirements() -> Vec<RustVerificationRequirement> {
    requirements([(
        "profile_resolution",
        "updated responsibility hint or complete waiver rationale",
    )])
}

fn requirements<const N: usize>(
    requirements: [(&'static str, &'static str); N],
) -> Vec<RustVerificationRequirement> {
    requirements
        .into_iter()
        .map(|(key, description)| RustVerificationRequirement::new(key, description))
        .collect()
}
