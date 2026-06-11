use super::predicate::{
    PreparedSyntaxQueryPredicate, SyntaxQueryPredicate, SyntaxQueryPredicateOp,
    SyntaxQueryPredicateValue,
};
use super::prefilter::source_may_match_query;

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
        &[prepared],
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
        &[prepared],
    ));
}

#[test]
fn terms_skip_sources_before_parse() {
    assert!(!source_may_match_query(
        "pub fn alpha() {}",
        &["beta".to_string()],
        &[],
    ));
}
