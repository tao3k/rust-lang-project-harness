//! Stable task fingerprints.

use std::path::Path;

use super::{RustVerificationEvidence, RustVerificationRequirement, RustVerificationTaskKind};

pub(crate) struct VerificationFingerprintInput<'a> {
    pub(crate) kind: RustVerificationTaskKind,
    pub(crate) project_root: &'a Path,
    pub(crate) package_root: &'a Path,
    pub(crate) owner_path: &'a Path,
    pub(crate) line: Option<usize>,
    pub(crate) required_evidence: &'a [RustVerificationRequirement],
    pub(crate) evidence: &'a [RustVerificationEvidence],
    pub(crate) skill_contract_material: Option<&'a str>,
}

pub(crate) fn verification_task_fingerprint(source: VerificationFingerprintInput<'_>) -> String {
    let relative_package = source
        .package_root
        .strip_prefix(source.project_root)
        .unwrap_or(source.package_root)
        .display()
        .to_string()
        .replace('\\', "/");
    let relative_owner = source
        .owner_path
        .strip_prefix(source.package_root)
        .unwrap_or(source.owner_path)
        .display()
        .to_string()
        .replace('\\', "/");
    let package_label = if relative_package.is_empty() {
        "."
    } else {
        relative_package.as_str()
    };
    let mut material = format!(
        "kind={};package={package_label};owner={relative_owner};",
        source.kind.as_str()
    );
    if let Some(line) = source.line {
        material.push_str(&format!("line={line};"));
    }
    for requirement in source.required_evidence {
        material.push_str("requires=");
        material.push_str(&requirement.key);
        material.push(';');
    }
    for fact in source.evidence {
        material.push_str(&fact.label);
        material.push('=');
        material.push_str(&fact.value);
        material.push(';');
    }
    if let Some(contract_material) = source.skill_contract_material {
        material.push_str("skill_contract=");
        material.push_str(contract_material);
        material.push(';');
    }
    format!("rustv:{:016x}", fnv1a_64(material.as_bytes()))
}

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
