mod fixtures;
mod flow;
mod prime;
mod public_fixtures;
mod public_large_flow;

const FORBIDDEN_PRIME_PATTERNS: &[&str] =
    &["--compact", "intent=", "subagent-plan", "|hit ", "Modules:"];

const FORBIDDEN_FLOW_PATTERNS: &[&str] =
    &["--compact", "intent=", "subagent-plan", "|hit ", "Modules:"];

fn assert_lab_packet(
    name: &str,
    rendered: &str,
    max_lines: usize,
    required: &[&str],
    forbidden: &[&str],
) {
    let line_count = rendered.lines().count();
    assert!(
        line_count <= max_lines,
        "{} exceeded max_lines={} with {} lines:\n{}",
        name,
        max_lines,
        line_count,
        rendered
    );
    for required in required {
        assert!(
            rendered.contains(required),
            "{} missing required fragment {required:?} in:\n{}",
            name,
            rendered
        );
    }
    for forbidden in forbidden {
        assert!(
            !rendered.contains(forbidden),
            "{} contained forbidden fragment {forbidden:?} in:\n{}",
            name,
            rendered
        );
    }
}
