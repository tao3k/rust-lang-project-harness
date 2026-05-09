//! Verification task construction and resolution helpers.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::fingerprint::{VerificationFingerprintInput, verification_task_fingerprint};
use super::profile::task_contract_for_profile;
use super::{
    RustVerificationEvidence, RustVerificationPolicy, RustVerificationProfileHint,
    RustVerificationReceipt, RustVerificationReceiptStatus, RustVerificationResolutionNote,
    RustVerificationSkillBinding, RustVerificationSkillDescriptor, RustVerificationTask,
    RustVerificationTaskContract, RustVerificationTaskKind, RustVerificationTaskState,
    RustVerificationWaiver,
};

pub(super) struct VerificationTaskSpec {
    pub(super) kind: RustVerificationTaskKind,
    pub(super) owner_path: PathBuf,
    pub(super) owner_namespace: Vec<String>,
    pub(super) line: Option<usize>,
    pub(super) reason: String,
    pub(super) contract: RustVerificationTaskContract,
    pub(super) evidence: Vec<RustVerificationEvidence>,
}

pub(super) struct ProfileReviewTaskSpec<'a> {
    pub(super) owner_path: PathBuf,
    pub(super) owner_namespace: Vec<String>,
    pub(super) reason: &'static str,
    pub(super) evidence: Vec<RustVerificationEvidence>,
    pub(super) hint: Option<&'a RustVerificationProfileHint>,
}

pub(super) fn new_profile_review_task(
    project_root: &Path,
    package_root: &Path,
    spec: ProfileReviewTaskSpec<'_>,
    policy: &RustVerificationPolicy,
) -> RustVerificationTask {
    new_skill_task(
        project_root,
        package_root,
        VerificationTaskSpec {
            kind: RustVerificationTaskKind::ResponsibilityReview,
            owner_path: spec.owner_path,
            owner_namespace: spec.owner_namespace,
            line: None,
            reason: spec.reason.to_string(),
            contract: task_contract_for_profile(
                policy,
                spec.hint,
                RustVerificationTaskKind::ResponsibilityReview,
            ),
            evidence: spec.evidence,
        },
        policy,
    )
}

pub(super) fn new_skill_task(
    project_root: &Path,
    package_root: &Path,
    spec: VerificationTaskSpec,
    policy: &RustVerificationPolicy,
) -> RustVerificationTask {
    let skill_binding = skill_binding_for_task(policy, spec.kind);
    let skill_descriptor = skill_binding
        .as_ref()
        .and_then(|binding| skill_descriptor_for_binding(policy, binding));
    let skill_contract_ref = skill_descriptor.map(RustVerificationSkillDescriptor::compact_label);
    let skill_contract_material =
        skill_descriptor.map(RustVerificationSkillDescriptor::fingerprint_material);
    let mut evidence = spec.evidence;
    if let Some(binding) = &skill_binding {
        evidence.push(RustVerificationEvidence::new(
            "skill",
            binding.compact_label(),
        ));
    }
    let fingerprint = verification_task_fingerprint(VerificationFingerprintInput {
        kind: spec.kind,
        project_root,
        package_root,
        owner_path: &spec.owner_path,
        line: spec.line,
        required_evidence: &spec.contract.required_evidence,
        evidence: &evidence,
        skill_contract_material: skill_contract_material.as_deref(),
    });
    let mut task = RustVerificationTask {
        fingerprint,
        kind: spec.kind,
        state: RustVerificationTaskState::Pending,
        package_root: package_root.to_path_buf(),
        owner_path: spec.owner_path,
        owner_namespace: spec.owner_namespace,
        line: spec.line,
        phase: spec.contract.phase,
        reason: spec.reason,
        required_receipt: spec.contract.required_receipt,
        skill_binding,
        skill_contract_ref,
        required_evidence: spec.contract.required_evidence,
        evidence,
        resolution_notes: Vec::new(),
        receipt_summary: None,
        receipt_evidence: Vec::new(),
        receipt_evidence_uri: None,
        receipt_observed_at: None,
        waiver_reason: None,
    };
    apply_task_resolution(&mut task, policy);
    task
}

pub(super) fn skill_descriptors_for_tasks(
    policy: &RustVerificationPolicy,
    tasks: &[RustVerificationTask],
) -> Vec<RustVerificationSkillDescriptor> {
    let mut descriptors = tasks
        .iter()
        .filter(|task| task.is_active())
        .filter_map(|task| task.skill_contract_ref.as_ref())
        .filter_map(|contract_ref| policy.skill_descriptors.get(contract_ref))
        .cloned()
        .collect::<Vec<_>>();
    descriptors.sort_by_key(RustVerificationSkillDescriptor::compact_label);
    descriptors.dedup_by_key(|descriptor| descriptor.compact_label());
    descriptors
}

pub(super) fn push_task(
    tasks: &mut BTreeMap<String, RustVerificationTask>,
    policy: &RustVerificationPolicy,
    task: RustVerificationTask,
) {
    if policy.disabled_task_kinds.contains(&task.kind) {
        return;
    }
    tasks.entry(task.fingerprint.clone()).or_insert(task);
}

fn skill_binding_for_task(
    policy: &RustVerificationPolicy,
    kind: RustVerificationTaskKind,
) -> Option<RustVerificationSkillBinding> {
    policy
        .skill_bindings
        .get(&kind)
        .filter(|binding| binding.is_configured())
        .cloned()
}

fn skill_descriptor_for_binding<'a>(
    policy: &'a RustVerificationPolicy,
    binding: &RustVerificationSkillBinding,
) -> Option<&'a RustVerificationSkillDescriptor> {
    policy
        .skill_descriptors
        .get(&binding.compact_label())
        .or_else(|| policy.skill_descriptors.get(&binding.skill_id))
}

fn apply_task_resolution(task: &mut RustVerificationTask, policy: &RustVerificationPolicy) {
    if let Some(receipt) = matching_receipt(task, policy, RustVerificationReceiptStatus::Passed) {
        task.state = RustVerificationTaskState::Satisfied;
        task.receipt_summary = Some(receipt_summary(receipt));
        task.receipt_evidence.clone_from(&receipt.evidence);
        task.receipt_evidence_uri.clone_from(&receipt.evidence_uri);
        task.receipt_observed_at.clone_from(&receipt.observed_at);
        return;
    }
    if let Some(waiver) = matching_waiver(task, policy) {
        task.state = RustVerificationTaskState::Waived;
        task.waiver_reason = Some(waiver.reason.clone());
        return;
    }
    if let Some(waiver) = incomplete_matching_waiver(task, policy) {
        task.resolution_notes
            .push(RustVerificationResolutionNote::new(
                "waiver",
                incomplete_waiver_detail(waiver),
            ));
    }
    if let Some(receipt) = matching_receipt(task, policy, RustVerificationReceiptStatus::Failed) {
        task.state = RustVerificationTaskState::Failed;
        task.receipt_summary = Some(receipt_summary(receipt));
        task.receipt_evidence.clone_from(&receipt.evidence);
        task.receipt_evidence_uri.clone_from(&receipt.evidence_uri);
        task.receipt_observed_at.clone_from(&receipt.observed_at);
    }
}

fn matching_receipt<'a>(
    task: &RustVerificationTask,
    policy: &'a RustVerificationPolicy,
    status: RustVerificationReceiptStatus,
) -> Option<&'a RustVerificationReceipt> {
    policy.receipts.iter().find(|receipt| {
        receipt.task_fingerprint == task.fingerprint
            && receipt.kind == task.kind
            && receipt.status == status
    })
}

fn matching_waiver<'a>(
    task: &RustVerificationTask,
    policy: &'a RustVerificationPolicy,
) -> Option<&'a RustVerificationWaiver> {
    policy
        .waivers
        .iter()
        .find(|waiver| waiver.task_fingerprint == task.fingerprint && waiver.is_complete())
}

fn incomplete_matching_waiver<'a>(
    task: &RustVerificationTask,
    policy: &'a RustVerificationPolicy,
) -> Option<&'a RustVerificationWaiver> {
    policy
        .waivers
        .iter()
        .find(|waiver| waiver.task_fingerprint == task.fingerprint && !waiver.is_complete())
}

fn incomplete_waiver_detail(waiver: &RustVerificationWaiver) -> String {
    let mut missing_fields = Vec::new();
    if waiver.owner.trim().is_empty() {
        missing_fields.push("owner");
    }
    if waiver.reason.trim().is_empty() {
        missing_fields.push("reason");
    }
    if waiver.expires_at.trim().is_empty() {
        missing_fields.push("expires_at");
    }
    format!("incomplete: missing {}", missing_fields.join(", "))
}

fn receipt_summary(receipt: &RustVerificationReceipt) -> String {
    receipt.evidence_uri.as_ref().map_or_else(
        || receipt.summary.clone(),
        |uri| format!("{} ({uri})", receipt.summary),
    )
}
