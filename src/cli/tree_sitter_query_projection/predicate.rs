//! Shared tree-sitter query predicate types and compiled operands.

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

impl SyntaxQueryPredicateOp {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::AnyEq => "any-eq",
            Self::AnyOf => "any-of",
            Self::Match => "match",
            Self::AnyMatch => "any-match",
            Self::NotEq => "not-eq",
            Self::NotMatch => "not-match",
        }
    }
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

pub(super) struct PreparedSyntaxQueryPredicate<'a> {
    pub(super) predicate: &'a SyntaxQueryPredicate,
    pub(super) regexes: Vec<regex::Regex>,
}
