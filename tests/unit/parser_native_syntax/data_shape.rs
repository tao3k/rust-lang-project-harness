use std::fs;

use tempfile::TempDir;

use crate::parser::parse_rust_file;

#[test]
fn native_syntax_facts_record_public_enum_tuple_variant_fields() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "pub enum DomainEvent {\n\
         \tUserLoaded(String, usize, bool),\n\
         \tTyped(UserId, UserCount),\n\
         \t#[cfg(test)]\n\
         \tFixture(String, bool),\n\
         }\n\
         enum PrivateEvent { UserLoaded(String, usize) }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let fields = module.syntax_facts.public_enum_tuple_variant_fields;
    assert_eq!(
        fields
            .iter()
            .map(|field| {
                (
                    field.enum_name.as_str(),
                    field.variant_name.as_str(),
                    field.field_index,
                    field.primitive_contract_type.as_deref(),
                    field.flag_contract_type.as_deref(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("DomainEvent", "UserLoaded", 0, Some("String"), None),
            ("DomainEvent", "UserLoaded", 1, Some("usize"), None),
            ("DomainEvent", "UserLoaded", 2, None, Some("bool")),
            ("DomainEvent", "Typed", 0, None, None),
            ("DomainEvent", "Typed", 1, None, None),
            ("DomainEvent", "Fixture", 0, Some("String"), None),
            ("DomainEvent", "Fixture", 1, None, Some("bool")),
        ]
    );
    assert_eq!(fields[0].enum_line, 1);
    assert_eq!(fields[0].variant_line, 2);
    assert!(fields[5].is_test_context);
}

#[test]
fn native_syntax_facts_record_public_type_alias_carriers() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "pub type UserId = String;\n\
         pub type IncludeInactive = bool;\n\
         pub type DomainUserId = UserId;\n\
         type PrivateToken = String;\n\
         #[cfg(test)]\n\
         pub type FixtureId = String;\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let aliases = module.syntax_facts.public_type_aliases;
    assert_eq!(
        aliases
            .iter()
            .map(|alias| {
                (
                    alias.alias_name.as_str(),
                    alias.target_type_text.as_str(),
                    alias.primitive_contract_type.as_deref(),
                    alias.flag_contract_type.as_deref(),
                    alias.is_test_context,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("UserId", "String", Some("String"), None, false),
            ("IncludeInactive", "bool", None, Some("bool"), false),
            ("DomainUserId", "UserId", None, None, false),
            ("FixtureId", "String", Some("String"), None, true),
        ]
    );
    assert_eq!(aliases[0].line, 1);
}
