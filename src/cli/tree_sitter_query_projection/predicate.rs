use agent_semantic_tree_sitter_runtime::NativeQueryMatch;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(in crate::cli) enum SyntaxQueryPredicateOp {
    Eq,
    AnyEq,
    AnyOf,
    Match,
    AnyMatch,
    NotEq,
    NotMatch,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub(in crate::cli) enum SyntaxQueryPredicateValue {
    String(String),
    Capture(String),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub(in crate::cli) struct SyntaxQueryPredicate {
    pub(in crate::cli) op: SyntaxQueryPredicateOp,
    pub(in crate::cli) capture: String,
    pub(in crate::cli) values: Vec<SyntaxQueryPredicateValue>,
}

pub(super) fn native_query_match_predicates_match(
    query_match: &NativeQueryMatch,
    predicates: &[SyntaxQueryPredicate],
) -> Result<bool, String> {
    predicates.iter().try_fold(true, |matches, predicate| {
        if !matches {
            return Ok(false);
        }
        native_predicate_matches(query_match, predicate)
    })
}

fn native_predicate_matches(
    query_match: &NativeQueryMatch,
    predicate: &SyntaxQueryPredicate,
) -> Result<bool, String> {
    let captures = capture_values(query_match, &predicate.capture);
    if captures.is_empty() {
        return Ok(false);
    }
    let values = predicate_values(query_match, &predicate.values);
    match predicate.op {
        SyntaxQueryPredicateOp::Eq
        | SyntaxQueryPredicateOp::AnyEq
        | SyntaxQueryPredicateOp::AnyOf => Ok(captures
            .iter()
            .any(|capture| values.iter().any(|value| capture == value))),
        SyntaxQueryPredicateOp::Match | SyntaxQueryPredicateOp::AnyMatch => {
            let regexes = values
                .iter()
                .map(|value| {
                    regex::Regex::new(value).map_err(|error| {
                        format!(
                            "invalid tree-sitter query predicate regex `{value}` for {}: {error}",
                            predicate.capture
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(captures
                .iter()
                .any(|capture| regexes.iter().any(|regex| regex.is_match(capture))))
        }
        SyntaxQueryPredicateOp::NotEq => Ok(captures
            .iter()
            .all(|capture| values.iter().all(|value| capture != value))),
        SyntaxQueryPredicateOp::NotMatch => {
            let regexes = values
                .iter()
                .map(|value| {
                    regex::Regex::new(value).map_err(|error| {
                        format!(
                            "invalid tree-sitter query predicate regex `{value}` for {}: {error}",
                            predicate.capture
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(captures
                .iter()
                .all(|capture| regexes.iter().all(|regex| !regex.is_match(capture))))
        }
    }
}

fn capture_values<'a>(query_match: &'a NativeQueryMatch, capture: &str) -> Vec<&'a str> {
    query_match
        .captures
        .iter()
        .filter(|item| item.capture_name == capture)
        .map(|item| item.node.text.as_str())
        .collect()
}

fn predicate_values(
    query_match: &NativeQueryMatch,
    values: &[SyntaxQueryPredicateValue],
) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| match value {
            SyntaxQueryPredicateValue::String(value) => vec![value.clone()],
            SyntaxQueryPredicateValue::Capture(capture) => capture_values(query_match, capture)
                .into_iter()
                .map(ToString::to_string)
                .collect(),
        })
        .collect()
}
