//! Cheap source-level filters before native syntax projection parses a file.

use super::predicate::{
    PreparedSyntaxQueryPredicate, SyntaxQueryPredicate, SyntaxQueryPredicateOp,
    SyntaxQueryPredicateValue,
};

pub(super) fn source_may_match_query(
    source: &str,
    terms: &[String],
    predicates: &[PreparedSyntaxQueryPredicate<'_>],
) -> bool {
    if !terms_may_match(source, terms) {
        return false;
    }
    predicates.iter().all(|predicate| {
        let literals = positive_exact_string_literals(predicate.predicate);
        literals.is_empty()
            || literals
                .iter()
                .any(|literal| source.contains(literal.as_str()))
    })
}

fn terms_may_match(source: &str, terms: &[String]) -> bool {
    if terms.is_empty() {
        return true;
    }
    let source = source.to_ascii_lowercase();
    terms
        .iter()
        .map(|term| term.trim().to_ascii_lowercase())
        .filter(|term| !term.is_empty())
        .all(|term| source.contains(&term))
}

fn positive_exact_string_literals(predicate: &SyntaxQueryPredicate) -> Vec<String> {
    if !matches!(
        predicate.op,
        SyntaxQueryPredicateOp::Eq | SyntaxQueryPredicateOp::AnyEq | SyntaxQueryPredicateOp::AnyOf
    ) {
        return Vec::new();
    }
    predicate
        .values
        .iter()
        .filter_map(|value| match value {
            SyntaxQueryPredicateValue::String(value) if !value.is_empty() => Some(value.clone()),
            _ => None,
        })
        .collect()
}
