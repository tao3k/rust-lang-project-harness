//! Shared documentation marker helpers for agent-policy boundary rules.

pub(super) fn documented_agent_boundary(
    source: &str,
    one_based_line: usize,
    markers: &[&str],
) -> bool {
    let mut collected_docs = String::new();
    let lines = source.lines().collect::<Vec<_>>();
    let mut index = one_based_line.min(lines.len());
    let mut is_anchor_line = true;
    while index > 0 {
        index -= 1;
        let trimmed = lines[index].trim_start();
        if let Some(doc) = trimmed.strip_prefix("///") {
            collected_docs.push_str(doc);
            collected_docs.push('\n');
            is_anchor_line = false;
            continue;
        }
        if let Some(doc) = trimmed.strip_prefix("//!") {
            collected_docs.push_str(doc);
            collected_docs.push('\n');
            is_anchor_line = false;
            continue;
        }
        if trimmed.starts_with("#[") || trimmed.is_empty() {
            is_anchor_line = false;
            continue;
        }
        if is_anchor_line {
            is_anchor_line = false;
            continue;
        }
        break;
    }
    docs_contain_marker(&collected_docs, markers)
}

pub(super) fn module_doc_contains(source: &str, markers: &[&str]) -> bool {
    let mut collected_docs = String::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        if let Some(doc) = trimmed.strip_prefix("//!") {
            collected_docs.push_str(doc);
            collected_docs.push('\n');
            continue;
        }
        if trimmed.starts_with("#![") || trimmed.is_empty() {
            continue;
        }
        break;
    }
    docs_contain_marker(&collected_docs, markers)
}

fn docs_contain_marker(docs: &str, markers: &[&str]) -> bool {
    let normalized = docs.to_ascii_lowercase();
    markers
        .iter()
        .any(|marker| normalized.contains(&marker.to_ascii_lowercase()))
}
