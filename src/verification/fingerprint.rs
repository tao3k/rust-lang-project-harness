//! Stable task fingerprints.

use std::path::Path;

use super::{RustVerificationEvidence, RustVerificationTaskKind};

pub(crate) fn verification_task_fingerprint(
    kind: RustVerificationTaskKind,
    project_root: &Path,
    package_root: &Path,
    owner_path: &Path,
    line: Option<usize>,
    evidence: &[RustVerificationEvidence],
) -> String {
    let relative_package = package_root
        .strip_prefix(project_root)
        .unwrap_or(package_root)
        .display()
        .to_string()
        .replace('\\', "/");
    let relative_owner = owner_path
        .strip_prefix(package_root)
        .unwrap_or(owner_path)
        .display()
        .to_string()
        .replace('\\', "/");
    let package_label = if relative_package.is_empty() {
        "."
    } else {
        relative_package.as_str()
    };
    let mut input = format!(
        "kind={};package={package_label};owner={relative_owner};",
        kind.as_str()
    );
    if let Some(line) = line {
        input.push_str(&format!("line={line};"));
    }
    for fact in evidence {
        input.push_str(&fact.label);
        input.push('=');
        input.push_str(&fact.value);
        input.push(';');
    }
    format!("rustv:{:016x}", fnv1a_64(input.as_bytes()))
}

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
