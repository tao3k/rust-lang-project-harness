//! Fixture with a loop-local linear membership scan.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Candidate {
    pub id: u64,
    pub label: String,
}

pub fn selected_labels(allowed_ids: &[u64], candidates: &[Candidate]) -> Vec<String> {
    let mut labels = Vec::new();
    for candidate in candidates {
        if allowed_ids.iter().any(|id| *id == candidate.id) {
            labels.push(candidate.label.clone());
        }
    }
    labels
}
