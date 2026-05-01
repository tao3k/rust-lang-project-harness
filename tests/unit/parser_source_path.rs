use std::path::PathBuf;

use crate::parser::rust_source_path_facts;

#[test]
fn source_path_facts_include_file_stem_namespaces() {
    let root = PathBuf::from("workspace");
    let source_root = root.join("src");
    let path = root.join("src/domain/domain.rs");

    let facts = rust_source_path_facts(&root, std::slice::from_ref(&source_root), &[], &[], &path);

    assert_eq!(
        facts.namespace_components,
        vec![
            "src".to_string(),
            "domain".to_string(),
            "domain".to_string()
        ]
    );
    assert!(facts.repeated_namespace_segments.contains("domain"));
    assert_eq!(
        facts.repeated_namespace_branch,
        Some(["src", "domain", "domain"].iter().collect::<PathBuf>())
    );
}

#[test]
fn source_path_facts_identify_special_rust_entrypoints() {
    let root = PathBuf::from("workspace");
    let source_root = root.join("src");
    let package_paths = vec![root.join("build.rs"), root.join("examples")];
    let source_paths = vec![source_root.clone()];

    let lib = rust_source_path_facts(
        &root,
        &source_paths,
        &[],
        &package_paths,
        &source_root.join("lib.rs"),
    );
    assert!(lib.is_crate_facade);
    assert!(lib.is_special_entrypoint);
    assert!(!lib.is_binary_entrypoint);

    let interface = rust_source_path_facts(
        &root,
        &source_paths,
        &[],
        &package_paths,
        &source_root.join("domain/mod.rs"),
    );
    assert!(interface.is_interface_mod);
    assert!(interface.is_special_entrypoint);

    let main = rust_source_path_facts(
        &root,
        &source_paths,
        &[],
        &package_paths,
        &source_root.join("main.rs"),
    );
    assert!(main.is_binary_entrypoint);
    assert!(main.is_special_entrypoint);

    let bin_file = rust_source_path_facts(
        &root,
        &source_paths,
        &[],
        &package_paths,
        &source_root.join("bin/tool.rs"),
    );
    assert!(bin_file.is_binary_entrypoint);

    let bin_main = rust_source_path_facts(
        &root,
        &source_paths,
        &[],
        &package_paths,
        &source_root.join("bin/tool/main.rs"),
    );
    assert!(bin_main.is_binary_entrypoint);

    let build_script = rust_source_path_facts(
        &root,
        &source_paths,
        &[],
        &package_paths,
        &root.join("build.rs"),
    );
    assert!(build_script.is_package_entrypoint);
    assert!(build_script.is_build_script_entrypoint);

    let example = rust_source_path_facts(
        &root,
        &source_paths,
        &[],
        &package_paths,
        &root.join("examples/demo.rs"),
    );
    assert!(!example.is_package_entrypoint);
    assert!(!example.is_build_script_entrypoint);
}
