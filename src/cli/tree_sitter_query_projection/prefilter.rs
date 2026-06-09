//! Cheap source-level filters before native syntax projection parses a file.

use super::core::{
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

#[cfg(test)]
mod tests {
    use super::*;

    fn exact_predicate(value: &str) -> SyntaxQueryPredicate {
        SyntaxQueryPredicate {
            op: SyntaxQueryPredicateOp::Eq,
            capture: "function.name".to_string(),
            values: vec![SyntaxQueryPredicateValue::String(value.to_string())],
        }
    }

    #[test]
    fn exact_predicate_literal_can_skip_irrelevant_source() {
        let predicate = exact_predicate("needle_target");
        let prepared = PreparedSyntaxQueryPredicate {
            predicate: &predicate,
            regexes: Vec::new(),
        };

        assert!(!source_may_match_query(
            "pub fn unrelated() {}",
            &[],
            &[prepared]
        ));
    }

    #[test]
    fn exact_predicate_literal_keeps_candidate_source() {
        let predicate = exact_predicate("needle_target");
        let prepared = PreparedSyntaxQueryPredicate {
            predicate: &predicate,
            regexes: Vec::new(),
        };

        assert!(source_may_match_query(
            "pub fn needle_target() {}",
            &[],
            &[prepared]
        ));
    }

    #[test]
    fn terms_skip_sources_before_parse() {
        assert!(!source_may_match_query(
            "pub fn alpha() {}",
            &["beta".to_string()],
            &[]
        ));
    }
}
