use super::{ParseArtifact, ParseArtifactItem};

fn artifact_with_kind(kind: &str) -> ParseArtifact {
    ParseArtifact::new(
        "same-v1-key",
        vec![ParseArtifactItem {
            kind: kind.to_string(),
            name: "target".to_string(),
            start: 0,
            end: 1,
        }],
        Vec::new(),
    )
}

#[test]
fn v1_rejects_legacy_fn_kind_so_live_parse_can_rebuild() {
    assert!(!artifact_with_kind("fn").matches_key("same-v1-key"));
}

#[test]
fn v1_accepts_canonical_function_kind() {
    assert!(artifact_with_kind("function").matches_key("same-v1-key"));
}
