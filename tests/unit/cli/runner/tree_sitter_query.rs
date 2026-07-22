use super::{exact_owner_path, inline_eq_predicate, predicate_function_name};

#[test]
fn predicate_plan_extracts_exact_function_name() {
    let predicate = r#"[{"capture":"function.name","op":"eq","values":[{"kind":"string","value":"parse_query"}]}]"#;
    assert_eq!(
        predicate_function_name(predicate).expect("parse predicate"),
        Some("parse_query".to_string())
    );
    assert_eq!(
        inline_eq_predicate(r#"(#eq? @function.name "parse_query")"#),
        Some("parse_query".to_string())
    );
}

#[test]
fn selector_must_remain_workspace_relative() {
    assert_eq!(
        exact_owner_path("rust://src/cli/query.rs#item/function/parse_query")
            .expect("canonical selector"),
        "src/cli/query.rs"
    );
    assert!(exact_owner_path("../outside.rs").is_err());
    assert!(exact_owner_path("/absolute.rs").is_err());
}
