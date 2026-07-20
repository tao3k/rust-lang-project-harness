use std::path::Path;

use crate::parser::ParsedRustModule;
use crate::search::hits::{SearchHit, sort_search_hits_by_recency};

pub(super) fn api_hits(
    package_root: &Path,
    parsed_modules: &[ParsedRustModule],
    hits: &[SearchHit],
    item_name: &str,
) -> Vec<SearchHit> {
    let mut api_hits = hits
        .iter()
        .filter(|hit| hit.name == item_name)
        .filter(|hit| hit_is_public_api(parsed_modules, hit))
        .cloned()
        .collect::<Vec<_>>();
    sort_search_hits_by_recency(package_root, &mut api_hits);
    api_hits
}

pub(super) fn api_fact_fields_for_hit(
    parsed_modules: &[ParsedRustModule],
    hit: &SearchHit,
) -> String {
    let Some(module) = parsed_modules
        .iter()
        .find(|module| module.report.path == hit.path)
    else {
        return String::new();
    };
    if let Some(callable) = module
        .syntax_facts
        .public_api_callables
        .iter()
        .find(|callable| {
            callable.line == hit.line && callable.name == hit.name && !callable.is_test_context
        })
    {
        return format!(
            " apiKind={} public={} docs={}",
            callable.kind, callable.is_public, callable.has_doc
        );
    }
    let Some(item) = module
        .syntax_facts
        .top_level_items
        .iter()
        .find(|item| item.line == hit.line && item.name.as_deref() == Some(hit.name.as_str()))
    else {
        return String::new();
    };
    format!(
        " apiKind={} public={} docs={}",
        item.kind, item.is_public, item.has_doc
    )
}

pub(super) fn compact_api_value(value: &str) -> String {
    let mut compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    for (spaced, canonical) in [
        (" :: ", "::"),
        (" < ", "<"),
        (" > ", ">"),
        (" ( ", "("),
        (" ) ", ")"),
        (" [ ", "["),
        (" ] ", "]"),
        (" , ", ","),
        (" ; ", ";"),
    ] {
        compact = compact.replace(spaced, canonical);
    }
    for (spaced, canonical) in [
        (":: ", "::"),
        (" ::", "::"),
        ("< ", "<"),
        (" <", "<"),
        ("> ", ">"),
        (" >", ">"),
        ("( ", "("),
        (" (", "("),
        (") ", ")"),
        (" )", ")"),
        ("[ ", "["),
        (" [", "["),
        ("] ", "]"),
        (" ]", "]"),
        (", ", ","),
        (" ,", ","),
        ("; ", ";"),
        (" ;", ";"),
    ] {
        compact = compact.replace(spaced, canonical);
    }
    compact
}

fn hit_is_public_api(parsed_modules: &[ParsedRustModule], hit: &SearchHit) -> bool {
    parsed_modules
        .iter()
        .find(|module| module.report.path == hit.path)
        .is_some_and(|module| {
            module
                .syntax_facts
                .public_api_callables
                .iter()
                .any(|callable| {
                    !callable.is_test_context
                        && callable.line == hit.line
                        && callable.name == hit.name
                        && (callable.is_public || callable.has_doc)
                })
                || module.syntax_facts.top_level_items.iter().any(|item| {
                    item.line == hit.line
                        && item.name.as_deref() == Some(hit.name.as_str())
                        && (item.is_public || item.has_doc)
                })
        })
}
