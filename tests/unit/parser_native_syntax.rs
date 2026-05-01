use std::fs;

use tempfile::TempDir;

use crate::parser::{RustUseGlobScopeKind, parse_rust_file};

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
fn native_syntax_facts_record_glob_scope_kinds() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("lib.rs");
    fs::write(
        &source,
        "use crate::gateway::studio::studio_repo_sync_api_tests::*;\n\
         use super::*;\n\
         use external::prelude::*;\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let glob_scopes = module
        .syntax_facts
        .use_statements
        .iter()
        .flat_map(|use_statement| &use_statement.glob_imports)
        .map(|glob| (glob.rendered_path(), glob.scope_kind))
        .collect::<Vec<_>>();
    assert_eq!(
        glob_scopes,
        vec![
            (
                "crate::gateway::studio::studio_repo_sync_api_tests::*".to_string(),
                RustUseGlobScopeKind::CrateOwner,
            ),
            ("super::*".to_string(), RustUseGlobScopeKind::ParentScope,),
            (
                "external::prelude::*".to_string(),
                RustUseGlobScopeKind::External,
            ),
        ]
    );
}

#[test]
fn native_syntax_facts_record_deep_relative_scope_imports() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("domain.rs");
    fs::write(
        &source,
        "pub use super::{super::MissingOwner, sibling::Thing};\n\
         use self::leaf::Leaf;\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let deep_relative_imports = module
        .syntax_facts
        .use_statements
        .iter()
        .flat_map(|use_statement| &use_statement.deep_relative_imports)
        .map(|import| (import.rendered_path(), import.parent_hops))
        .collect::<Vec<_>>();
    assert_eq!(
        deep_relative_imports,
        vec![("super::super::MissingOwner".to_string(), 2)]
    );
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

#[test]
fn native_syntax_facts_record_public_function_returns() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "pub enum UserError {}\n\
         pub fn load_user() -> anyhow::Result<String> { todo!() }\n\
         pub fn save_user() -> Result<(), Box<dyn std::error::Error + Send + Sync>> { todo!() }\n\
         pub fn typed_user() -> Result<(), UserError> { todo!() }\n\
         fn private_user() -> anyhow::Result<()> { todo!() }\n\
         #[cfg(test)]\n\
         pub fn fixture_user() -> eyre::Result<()> { todo!() }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let returns = module.syntax_facts.public_function_returns;
    assert_eq!(
        returns
            .iter()
            .map(|return_fact| return_fact.function_name.as_str())
            .collect::<Vec<_>>(),
        vec!["load_user", "save_user", "typed_user", "fixture_user"]
    );
    assert_eq!(
        returns[0].application_error_boundary.as_deref(),
        Some("anyhow::Result")
    );
    assert_eq!(
        returns[1].application_error_boundary.as_deref(),
        Some("Result<_, Box<dyn Error>>")
    );
    assert_eq!(returns[2].application_error_boundary, None);
    assert_eq!(
        returns[3].application_error_boundary.as_deref(),
        Some("eyre::Result")
    );
    assert!(returns[3].is_test_context);
}
