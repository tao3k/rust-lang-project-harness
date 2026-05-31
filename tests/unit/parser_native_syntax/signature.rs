use std::fs;

use tempfile::TempDir;

use crate::parser::parse_rust_file;

#[test]
fn native_syntax_facts_record_public_function_params() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "pub struct UserId(String);\n\
         pub fn load_user(user_id: String, id: Option<u64>, count: usize, typed: UserId, enabled: bool, cached: Option<bool>) {}\n\
         fn private_user(user_id: String) {}\n\
         #[cfg(test)]\n\
         pub fn fixture_user(user_id: String) {}\n\
         pub struct Loader;\n\
         impl Loader {\n\
         \tpub fn new(endpoint: String, retries: usize, enabled: bool) -> anyhow::Result<Self> { todo!() }\n\
         \tfn private_new(endpoint: String) {}\n\
         \t#[cfg(test)]\n\
         \tpub fn fixture_new(enabled: bool) {}\n\
         }\n",
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
            ("load_user", "enabled"),
            ("load_user", "cached"),
            ("fixture_user", "user_id"),
            ("new", "endpoint"),
            ("new", "retries"),
            ("new", "enabled"),
            ("fixture_new", "enabled"),
        ]
    );
    assert_eq!(params[0].function_line, 2);
    assert_eq!(params[0].primitive_contract_type.as_deref(), Some("String"));
    assert_eq!(
        params[1].primitive_contract_type.as_deref(),
        Some("Option<u64>")
    );
    assert_eq!(params[2].primitive_contract_type.as_deref(), Some("usize"));
    assert_eq!(params[3].primitive_contract_type, None);
    assert_eq!(params[4].flag_contract_type.as_deref(), Some("bool"));
    assert_eq!(
        params[5].flag_contract_type.as_deref(),
        Some("Option<bool>")
    );
    assert!(params[6].is_test_context);
    assert_eq!(params[7].function_line, 8);
    assert_eq!(params[7].primitive_contract_type.as_deref(), Some("String"));
    assert_eq!(params[9].flag_contract_type.as_deref(), Some("bool"));
    assert!(params[10].is_test_context);
}

#[test]
fn native_syntax_facts_record_public_struct_fields() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "pub struct UserProfile {\n\
         \tpub user_id: String,\n\
         \tpub timeout_ms: u64,\n\
         \tpub include_inactive: bool,\n\
         \tprivate_note: String,\n\
         }\n\
         struct PrivateProfile { pub user_id: String }\n\
         #[cfg(test)]\n\
         pub struct FixtureProfile { pub user_id: String }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let fields = module.syntax_facts.public_struct_fields;
    assert_eq!(
        fields
            .iter()
            .map(|field| (field.struct_name.as_str(), field.field_name.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("UserProfile", "user_id"),
            ("UserProfile", "timeout_ms"),
            ("UserProfile", "include_inactive"),
            ("FixtureProfile", "user_id"),
        ]
    );
    assert_eq!(fields[0].struct_line, 1);
    assert_eq!(fields[0].primitive_contract_type.as_deref(), Some("String"));
    assert_eq!(fields[1].primitive_contract_type.as_deref(), Some("u64"));
    assert_eq!(fields[2].flag_contract_type.as_deref(), Some("bool"));
    assert!(fields[3].is_test_context);
}

#[test]
fn native_syntax_facts_record_public_enum_variant_fields() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "pub enum DomainEvent {\n\
         \tUserLoaded { user_id: String, request_id: String, include_inactive: bool },\n\
         \tIgnored,\n\
         \t#[cfg(test)]\n\
         \tFixture { user_id: String },\n\
         }\n\
         enum PrivateEvent { UserLoaded { user_id: String } }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let fields = module.syntax_facts.public_enum_variant_fields;
    assert_eq!(
        fields
            .iter()
            .map(|field| {
                (
                    field.enum_name.as_str(),
                    field.variant_name.as_str(),
                    field.field_name.as_str(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("DomainEvent", "UserLoaded", "user_id"),
            ("DomainEvent", "UserLoaded", "request_id"),
            ("DomainEvent", "UserLoaded", "include_inactive"),
            ("DomainEvent", "Fixture", "user_id"),
        ]
    );
    assert_eq!(fields[0].enum_line, 1);
    assert_eq!(fields[0].variant_line, 2);
    assert_eq!(fields[0].primitive_contract_type.as_deref(), Some("String"));
    assert_eq!(fields[1].primitive_contract_type.as_deref(), Some("String"));
    assert_eq!(fields[2].flag_contract_type.as_deref(), Some("bool"));
    assert!(fields[3].is_test_context);
}

#[test]
fn native_syntax_facts_record_public_type_generic_bounds() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "pub struct Cache<T: Clone + std::fmt::Debug, U>\n\
         where\n\
         \tU: Default,\n\
         {\n\
         \tpub value: T,\n\
         \tpub fallback: U,\n\
         }\n\
         pub enum Event<T: serde::Serialize> { Item(T) }\n\
         struct Private<T: Clone> { value: T }\n\
         #[cfg(test)]\n\
         pub struct Fixture<T: Clone> { pub value: T }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let bounds = module.syntax_facts.public_type_generic_bounds;
    assert_eq!(
        bounds
            .iter()
            .map(|bound| {
                (
                    bound.type_kind,
                    bound.type_name.as_str(),
                    bound.param_name.as_str(),
                    bound.bound_name.as_str(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("struct", "Cache", "T", "Clone"),
            ("struct", "Cache", "T", "Debug"),
            ("struct", "Cache", "U", "Default"),
            ("enum", "Event", "T", "Serialize"),
            ("struct", "Fixture", "T", "Clone"),
        ]
    );
    assert_eq!(bounds[0].type_line, 1);
    assert!(bounds[4].is_test_context);
}

#[test]
fn native_syntax_facts_record_public_function_returns() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "pub enum UserError {}\n\
         pub fn load_user() -> anyhow::Result<String> { todo!() }\n\
         pub async unsafe fn fallible_user() -> anyhow::Result<String> { todo!() }\n\
         pub fn save_user() -> Result<(), Box<dyn std::error::Error + Send + Sync>> { todo!() }\n\
         pub fn typed_user() -> Result<(), UserError> { todo!() }\n\
         fn private_user() -> anyhow::Result<()> { todo!() }\n\
         #[cfg(test)]\n\
         pub fn fixture_user() -> eyre::Result<()> { todo!() }\n\
         pub trait LoadApi { fn trait_connect(&self) -> Result<String, UserError>; }\n\
         pub struct Loader;\n\
         impl Loader {\n\
         \tpub async unsafe fn connect(&mut self) -> color_eyre::Result<Self> { todo!() }\n\
         }\n\
         impl LoadApi for Loader {\n\
         \tfn trait_connect(&self) -> Result<String, UserError> { todo!() }\n\
         }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let returns = module.syntax_facts.public_function_returns;
    assert_eq!(
        returns
            .iter()
            .map(|return_fact| return_fact.function_name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "load_user",
            "fallible_user",
            "save_user",
            "typed_user",
            "fixture_user",
            "connect",
            "trait_connect"
        ]
    );
    assert_eq!(
        returns[0].application_error_boundary.as_deref(),
        Some("anyhow::Result")
    );
    assert!(returns[1].is_async);
    assert!(returns[1].is_unsafe);
    assert_eq!(
        returns[1].application_error_boundary.as_deref(),
        Some("anyhow::Result")
    );
    assert_eq!(
        returns[2].application_error_boundary.as_deref(),
        Some("Result<_, Box<dyn Error>>")
    );
    assert_eq!(returns[3].application_error_boundary, None);
    assert_eq!(
        returns[4].application_error_boundary.as_deref(),
        Some("eyre::Result")
    );
    assert!(returns[4].is_test_context);
    assert!(returns[5].is_async);
    assert!(returns[5].is_unsafe);
    assert_eq!(returns[5].receiver.as_deref(), Some("&mut-self"));
    assert_eq!(
        returns[5].application_error_boundary.as_deref(),
        Some("color_eyre::Result")
    );
    assert_eq!(returns[5].impl_type.as_deref(), Some("Loader"));
    assert_eq!(returns[5].trait_path, None);
    assert_eq!(returns[6].receiver.as_deref(), Some("&self"));
    assert_eq!(returns[6].impl_type.as_deref(), Some("Loader"));
    assert_eq!(returns[6].trait_path.as_deref(), Some("LoadApi"));
}
