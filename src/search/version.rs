use semver::{Comparator, Version, VersionReq};

pub(super) fn version_requirement_matches_request(
    current_requirement: Option<&str>,
    requested_requirement: &str,
) -> bool {
    let Some(current_requirement) = current_requirement else {
        return false;
    };
    if current_requirement == requested_requirement {
        return true;
    }
    let Ok(current_requirement) = VersionReq::parse(current_requirement) else {
        return false;
    };
    let Ok(requested_requirement) = VersionReq::parse(requested_requirement) else {
        return false;
    };
    version_requirements_overlap(&current_requirement, &requested_requirement)
}

fn version_requirements_overlap(left: &VersionReq, right: &VersionReq) -> bool {
    candidate_versions(left)
        .into_iter()
        .chain(candidate_versions(right))
        .any(|version| left.matches(&version) && right.matches(&version))
}

fn candidate_versions(requirement: &VersionReq) -> Vec<Version> {
    let mut versions = vec![Version::new(0, 0, 0), Version::new(1, 0, 0)];
    for comparator in &requirement.comparators {
        push_comparator_versions(&mut versions, comparator);
    }
    versions.sort();
    versions.dedup();
    versions
}

fn push_comparator_versions(versions: &mut Vec<Version>, comparator: &Comparator) {
    let major = comparator.major;
    let minor = comparator.minor.unwrap_or(0);
    let patch = comparator.patch.unwrap_or(0);

    push_version(versions, major, minor, patch);
    push_version(versions, major, minor, patch.saturating_add(1));
    push_version(versions, major, minor.saturating_add(1), 0);
    push_version(versions, major.saturating_add(1), 0, 0);
    if major > 0 {
        push_version(
            versions,
            major - 1,
            u64::from(u16::MAX),
            u64::from(u16::MAX),
        );
    }
}

fn push_version(versions: &mut Vec<Version>, major: u64, minor: u64, patch: u64) {
    versions.push(Version::new(major, minor, patch));
}
