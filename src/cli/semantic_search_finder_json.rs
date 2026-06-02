use serde_json::{Value, json};

use super::semantic_search_json::SemanticSearchJsonOptions;

pub(super) fn fzf_finder(options: &SemanticSearchJsonOptions) -> Value {
    let mut finder_options = json!({
        "matchMode": fzf_match_mode(&options.fzf_args),
        "caseMode": fzf_case_mode(&options.fzf_args),
        "nativeArgs": options.fzf_args,
    });
    if let Some(scheme) = fzf_scheme(&options.fzf_args) {
        finder_options["scheme"] = json!(scheme);
    }
    json!({
        "engine": "fzf",
        "surface": "search-fzf",
        "pipelineId": "provider-fzf",
        "options": finder_options,
        "acceptedArgs": options.fzf_args,
        "rejectedArgs": []
    })
}

fn fzf_match_mode(args: &[String]) -> &'static str {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "--exact" | "-e"))
    {
        "exact"
    } else {
        "fuzzy"
    }
}

fn fzf_case_mode(args: &[String]) -> &'static str {
    if args.iter().any(|arg| arg == "+i") {
        "respect"
    } else {
        "ignore"
    }
}

fn fzf_scheme(args: &[String]) -> Option<&str> {
    args.iter().find_map(|arg| arg.strip_prefix("--scheme="))
}
