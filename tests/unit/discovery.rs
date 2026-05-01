use crate::discovery::glob_pattern_matches;

#[test]
fn workspace_member_glob_matches_forward_slash_relative_paths() {
    assert!(glob_pattern_matches("crates/*", "crates/member"));
    assert!(!glob_pattern_matches("crates/*", "crates/member/nested"));
}

#[test]
fn workspace_member_glob_matches_windows_relative_paths() {
    assert!(glob_pattern_matches("crates/*", r"crates\member"));
    assert!(!glob_pattern_matches("crates/*", r"crates\member\nested"));
}
