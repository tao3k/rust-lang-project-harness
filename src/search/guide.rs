//! Typed `search guide` surface for graph-reasoning agent flows.

struct SearchGuideProfile {
    id: &'static str,
    command: Option<&'static str>,
    args: &'static str,
    returns: &'static str,
}

const SEARCH_GUIDE_PROFILES: &[SearchGuideProfile] = &[
    SearchGuideProfile {
        id: "overview-prime",
        command: Some("search prime --view seeds"),
        args: "",
        returns: "owner/query/dependency/test/finding handles",
    },
    SearchGuideProfile {
        id: "owner-items",
        command: None,
        args: "owner:path query:term",
        returns: "symbol/item frontier",
    },
    SearchGuideProfile {
        id: "owner-tests",
        command: None,
        args: "owner:path",
        returns: "test frontier",
    },
    SearchGuideProfile {
        id: "query-deps",
        command: None,
        args: "query:term dependency:pkg",
        returns: "dependency usage owners/tests",
    },
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
    lines.push(format!("avoid={}", SEARCH_GUIDE_AVOID.join(",")));
    lines.push(String::new());
    lines.join("\n")
}
