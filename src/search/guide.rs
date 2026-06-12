//! Typed `search guide` surface for graph-reasoning agent flows.

struct SearchGuideProfile {
    id: &'static str,
    command: Option<&'static str>,
    args: &'static str,
    returns: &'static str,
}

const SEARCH_GUIDE_REASONING_PROFILES: &str =
    "owner-query,query-deps,owner-tests,finding-frontier,feature-cfg";

const SEARCH_GUIDE_PROFILES: &[SearchGuideProfile] = &[
    SearchGuideProfile {
        id: "overview-prime",
        command: Some("search prime --workspace . --view seeds"),
        args: "",
        returns: "owner/query/dependency/test/finding handles",
    },
    SearchGuideProfile {
        id: "owner-query",
        command: Some(
            "search reasoning owner-query --owner <owner-path> --query <term> --view seeds",
        ),
        args: "owner:path query:term",
        returns: "symbol/item frontier",
    },
    SearchGuideProfile {
        id: "owner-tests",
        command: Some("search reasoning owner-tests --owner <owner-path> --view seeds"),
        args: "owner:path",
        returns: "test frontier",
    },
    SearchGuideProfile {
        id: "query-deps",
        command: Some("search reasoning query-deps --query <term> --dependency <pkg> --view seeds"),
        args: "query:term dependency:pkg",
        returns: "dependency usage owners/tests",
    },
    SearchGuideProfile {
        id: "finding-frontier",
        command: Some(
            "search reasoning finding-frontier --query <finding-term> --owner <owner-path> --view seeds",
        ),
        args: "finding:term owner:path",
        returns: "affected owners/tests/verification actions",
    },
    SearchGuideProfile {
        id: "feature-cfg",
        command: Some("search reasoning feature-cfg --query <feature-name> --view seeds"),
        args: "feature:name",
        returns: "cfg gates/owners/verification surfaces",
    },
];

const SEARCH_GUIDE_ROUTES: &[SearchGuideProfile] = &[
    SearchGuideProfile {
        id: "path",
        command: None,
        args: "from:typed-node to:typed-node",
        returns: "shortest relation path",
    },
    SearchGuideProfile {
        id: "read-frontier",
        command: None,
        args: "range:path@start:end",
        returns: "symbol/window frontier without code",
    },
];

const SEARCH_GUIDE_AVOID: &[&str] = &[
    "raw-read",
    "manual-window-scan",
    "full-json",
    "natural-language-intent",
];

pub(crate) fn render_search_guide() -> String {
    let mut lines = vec!["[search-guide] protocol=search-guide.v1".to_string()];
    lines.push(format!(
        "|catalog reasoningProfiles={SEARCH_GUIDE_REASONING_PROFILES} entries={SEARCH_GUIDE_REASONING_PROFILES} routes=path,read-frontier"
    ));
    lines.push("profiles:".to_string());
    for profile in SEARCH_GUIDE_PROFILES {
        lines.push(format!("  {}:", profile.id));
        if let Some(command) = profile.command {
            lines.push(format!("    command={command}"));
        }
        if !profile.args.is_empty() {
            lines.push(format!("    args={}", profile.args));
        }
        lines.push(format!("    returns={}", profile.returns));
    }
    lines.push("routes:".to_string());
    for route in SEARCH_GUIDE_ROUTES {
        lines.push(format!("  {}:", route.id));
        if let Some(command) = route.command {
            lines.push(format!("    command={command}"));
        }
        if !route.args.is_empty() {
            lines.push(format!("    args={}", route.args));
        }
        lines.push(format!("    returns={}", route.returns));
    }
    lines.push(format!("avoid={}", SEARCH_GUIDE_AVOID.join(",")));
    lines.push(String::new());
    lines.join("\n")
}
