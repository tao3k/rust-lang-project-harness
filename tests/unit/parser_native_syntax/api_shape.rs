use std::fs;

use tempfile::TempDir;

use crate::parser::parse_rust_file;

#[test]
fn native_syntax_facts_record_public_tuple_api_surfaces() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("api.rs");
    fs::write(
        &source,
        "pub fn load_user(cursor: (String, usize, bool)) -> Result<(String, usize), LoadError> { todo!() }\n\
         pub fn typed_user(cursor: UserCursor) -> UserPage { todo!() }\n\
         fn private_user(cursor: (String, usize)) -> (String, usize) { todo!() }\n\
         #[cfg(test)]\n\
         pub fn fixture_user(cursor: (String, bool)) {}\n\
         pub struct Loader;\n\
         impl Loader {\n\
         \tpub fn read_window(window: Option<(String, u64)>) -> Self { todo!() }\n\
         }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let surfaces = module.syntax_facts.public_tuple_api_surfaces;
    assert_eq!(
        surfaces
            .iter()
            .map(|surface| {
                let element_contract_types = surface
                    .element_contract_types
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>();
                (
                    surface.function_name.as_str(),
                    surface.surface_name.as_str(),
                    element_contract_types,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (
                "load_user",
                "parameter `cursor`",
                vec!["String", "usize", "bool"]
            ),
            ("load_user", "return value", vec!["String", "usize"]),
            ("fixture_user", "parameter `cursor`", vec!["String", "bool"]),
            ("read_window", "parameter `window`", vec!["String", "u64"]),
        ]
    );
    assert_eq!(surfaces[0].function_line, 1);
    assert_eq!(surfaces[0].line, 1);
    assert_eq!(surfaces[0].type_text, "(String , usize , bool)");
    assert_eq!(
        surfaces[1].type_text,
        "Result < (String , usize) , LoadError >"
    );
    assert!(surfaces[2].is_test_context);
}
