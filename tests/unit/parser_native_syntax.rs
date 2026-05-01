use std::fs;

use tempfile::TempDir;

use crate::parser::parse_rust_file;

#[test]
fn native_syntax_facts_record_cfg_and_module_declaring_macros() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("lib.rs");
    fs::write(
        &source,
        "#[cfg(feature = \"fs\")]\ncompile_error!(\"fs is not supported here\");\ncfg_feature! {\n    pub(crate) mod optional;\n}\ncfg_macros! {\n    pub use crate::optional::Thing;\n}\ncfg_nested! {\n    cfg_inner! {\n        pub(crate) use crate::optional::Other;\n    }\n}\ncfg_bad! {\n    pub fn leaked() {}\n}\nmacro_rules! declare_mod { () => { mod generated; } }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let compile_error = module
        .syntax_facts
        .top_level_items
        .iter()
        .find(|item| item.macro_name.as_deref() == Some("compile_error"))
        .expect("compile_error macro fact");
    assert!(compile_error.has_cfg_attr);
    assert!(!compile_error.macro_declares_module);
    assert!(!compile_error.macro_body_is_facade_boundary);

    let cfg_feature = module
        .syntax_facts
        .top_level_items
        .iter()
        .find(|item| item.macro_name.as_deref() == Some("cfg_feature"))
        .expect("cfg_feature macro fact");
    assert!(!cfg_feature.has_cfg_attr);
    assert!(cfg_feature.macro_declares_module);
    assert!(cfg_feature.macro_body_is_facade_boundary);

    let cfg_macros = module
        .syntax_facts
        .top_level_items
        .iter()
        .find(|item| item.macro_name.as_deref() == Some("cfg_macros"))
        .expect("cfg_macros macro fact");
    assert!(!cfg_macros.macro_declares_module);
    assert!(cfg_macros.macro_body_is_facade_boundary);

    let cfg_nested = module
        .syntax_facts
        .top_level_items
        .iter()
        .find(|item| item.macro_name.as_deref() == Some("cfg_nested"))
        .expect("cfg_nested macro fact");
    assert!(!cfg_nested.macro_declares_module);
    assert!(cfg_nested.macro_body_is_facade_boundary);

    let cfg_bad = module
        .syntax_facts
        .top_level_items
        .iter()
        .find(|item| item.macro_name.as_deref() == Some("cfg_bad"))
        .expect("cfg_bad macro fact");
    assert!(!cfg_bad.macro_body_is_facade_boundary);

    let macro_rules = module
        .syntax_facts
        .top_level_items
        .iter()
        .find(|item| item.macro_name.as_deref() == Some("macro_rules"))
        .expect("macro_rules macro fact");
    assert!(macro_rules.macro_declares_module);
    assert!(!macro_rules.macro_body_is_facade_boundary);
}

#[test]
fn native_syntax_facts_record_public_function_params() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "pub struct UserId(String);\n\
         pub fn load_user(user_id: String, id: Option<u64>, count: usize, typed: UserId) {}\n\
         fn private_user(user_id: String) {}\n\
         #[cfg(test)]\n\
         pub fn fixture_user(user_id: String) {}\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let params = module.syntax_facts.public_function_params;
    assert_eq!(
        params
            .iter()
            .map(|param| (param.function_name.as_str(), param.param_name.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("load_user", "user_id"),
            ("load_user", "id"),
            ("load_user", "count"),
            ("load_user", "typed"),
            ("fixture_user", "user_id"),
        ]
    );
    assert_eq!(params[0].primitive_contract_type.as_deref(), Some("String"));
    assert_eq!(
        params[1].primitive_contract_type.as_deref(),
        Some("Option<u64>")
    );
    assert_eq!(params[2].primitive_contract_type.as_deref(), Some("usize"));
    assert_eq!(params[3].primitive_contract_type, None);
    assert!(params[4].is_test_context);
}
