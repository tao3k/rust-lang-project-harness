//! Fixture with membership indexed before traversal.

use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Candidate {
    pub id: u64,
    pub label: String,
}

pub fn selected_labels(allowed_ids: &[u64], candidates: &[Candidate]) -> Vec<String> {
    let allowed = allowed_ids.iter().copied().collect::<BTreeSet<_>>();
    candidates
        .iter()
        .filter(|candidate| allowed.contains(&candidate.id))
        .map(|candidate| candidate.label.clone())
        .collect()
}
