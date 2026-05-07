//! Public data model for parser-derived verification profile candidates.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::verification::{
    RustOwnerResponsibility, RustVerificationEvidence, RustVerificationProfileHint,
    RustVerificationTaskKind,
};

/// Whether a parser-suggested verification profile is already configured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustVerificationProfileCandidateState {
    /// No matching profile hint exists for this parser owner.
    MissingProfile,
    /// A profile hint exists, but parser facts suggest additional responsibilities.
    ProfileDrift,
    /// A profile hint covers all parser-suggested responsibilities.
    Configured,
}

impl RustVerificationProfileCandidateState {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::MissingProfile => "missing_profile",
            Self::ProfileDrift => "profile_drift",
            Self::Configured => "configured",
        }
    }

    const fn requires_action(self) -> bool {
        matches!(self, Self::MissingProfile | Self::ProfileDrift)
    }
}

/// Searchable responsibility-profile candidates derived from parser facts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationProfileIndex {
    /// Root used to compact owner paths in agent renders.
    #[serde(default, skip_serializing_if = "path_buf_is_empty")]
    pub project_root: PathBuf,
    /// Parser-suggested owner profile candidates.
    pub candidates: Vec<RustVerificationProfileCandidate>,
    /// Number of profile hints configured before parser candidates were rendered.
    #[serde(default, skip_serializing_if = "usize_is_zero")]
    pub configured_profile_hint_count: usize,
}

impl RustVerificationProfileIndex {
    /// Return whether no owner still needs profile configuration.
    #[must_use]
    pub fn is_clear(&self) -> bool {
        self.active_candidates().is_empty()
    }

    /// Return whether parser facts found owners before any profile was configured.
    #[must_use]
    pub fn needs_profile_configuration(&self) -> bool {
        self.configured_profile_hint_count == 0 && !self.active_candidates().is_empty()
    }

    /// Return candidates that still need agent action.
    #[must_use]
    pub fn active_candidates(&self) -> Vec<&RustVerificationProfileCandidate> {
        self.candidates
            .iter()
            .filter(|candidate| candidate.requires_action())
            .collect()
    }

    /// Return candidates owned by one package root.
    #[must_use]
    pub fn candidates_for_package(
        &self,
        package_root: impl AsRef<Path>,
    ) -> Vec<&RustVerificationProfileCandidate> {
        let package_root = package_root.as_ref();
        self.candidates
            .iter()
            .filter(|candidate| {
                candidate.package_root == package_root
                    || candidate
                        .package_root
                        .strip_prefix(&self.project_root)
                        .is_ok_and(|relative| relative == package_root)
            })
            .collect()
    }

    /// Return suggested profile hints for candidates that still need action.
    #[must_use]
    pub fn active_profile_hints(&self) -> Vec<RustVerificationProfileHint> {
        self.active_candidates()
            .into_iter()
            .map(RustVerificationProfileCandidate::to_profile_hint)
            .collect()
    }
}

/// One owner profile candidate an Agent can turn into config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationProfileCandidate {
    /// Cargo package root that owns the parser facts.
    pub package_root: PathBuf,
    /// Owner module path.
    pub owner_path: PathBuf,
    /// Recommended path to use in `RustVerificationProfileHint`.
    pub hint_path: PathBuf,
    /// Parser-derived owner namespace.
    pub owner_namespace: Vec<String>,
    /// Whether the current config covers this candidate.
    pub state: RustVerificationProfileCandidateState,
    /// Responsibilities suggested by parser facts.
    pub suggested_responsibilities: BTreeSet<RustOwnerResponsibility>,
    /// Responsibilities already configured by a matching profile hint.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub configured_responsibilities: BTreeSet<RustOwnerResponsibility>,
    /// Effective task kinds implied by the suggested responsibilities and policy mapping.
    pub suggested_task_kinds: BTreeSet<RustVerificationTaskKind>,
    /// Compact parser facts behind the suggestion.
    pub evidence: Vec<RustVerificationEvidence>,
}

impl RustVerificationProfileCandidate {
    /// Return whether this candidate still needs agent action.
    #[must_use]
    pub const fn requires_action(&self) -> bool {
        self.state.requires_action()
    }

    /// Convert this candidate into a profile hint using the recommended path.
    #[must_use]
    pub fn to_profile_hint(&self) -> RustVerificationProfileHint {
        RustVerificationProfileHint::new(
            self.hint_path.clone(),
            self.suggested_responsibilities.iter().copied(),
        )
    }
}

fn path_buf_is_empty(path: &Path) -> bool {
    path.as_os_str().is_empty()
}

fn usize_is_zero(value: &usize) -> bool {
    *value == 0
}
