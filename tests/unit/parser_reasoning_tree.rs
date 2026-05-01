use std::fs;

use tempfile::TempDir;

use crate::RustProjectHarnessScope;
use crate::parser::{parse_rust_file, rust_reasoning_tree_facts};

#[test]
fn reasoning_tree_interprets_modules_owners_and_child_edges() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let src = root.join("src");
    fs::create_dir_all(src.join("domain")).expect("create source tree");
    fs::write(src.join("lib.rs"), "//! Crate facade.\nmod domain;\n").expect("write lib");
    fs::write(src.join("domain.rs"), "//! Domain branch.\nmod leaf;\n").expect("write domain");
    fs::write(src.join("domain/leaf.rs"), "//! Domain leaf.\n").expect("write leaf");

    let modules = [
        parse_rust_file(&src.join("lib.rs")),
        parse_rust_file(&src.join("domain.rs")),
        parse_rust_file(&src.join("domain/leaf.rs")),
    ];
    let scope = RustProjectHarnessScope {
        project_root: root.to_path_buf(),
        source_paths: vec![src.clone()],
        test_paths: Vec::new(),
        package_paths: vec![root.join("build.rs")],
        fallback_paths: vec![root.to_path_buf()],
    };

    let reasoning_tree = rust_reasoning_tree_facts(&scope, &modules);
    assert_eq!(reasoning_tree.package_root, root);
    assert_eq!(reasoning_tree.source_roots, vec![src.clone()]);
    assert_eq!(
        reasoning_tree.package_entrypoints,
        vec![root.join("build.rs")]
    );

    let lib = reasoning_tree
        .module(&src.join("lib.rs"))
        .expect("lib facts");
    assert!(lib.is_source_module);
    assert!(lib.is_module_tree_root);
    assert!(lib.source_path.is_crate_facade);
    assert_eq!(lib.declared_child_paths, vec![src.join("domain.rs")]);

    let branch = reasoning_tree
        .module(&src.join("domain.rs"))
        .expect("branch facts");
    assert_eq!(
        branch.source_path.namespace_components,
        vec!["src".to_string(), "domain".to_string()]
    );
    assert_eq!(
        branch.declared_child_paths,
        vec![src.join("domain/leaf.rs")]
    );

    assert!(reasoning_tree.shadowed_module_sources.is_empty());
    assert!(reasoning_tree.unreachable_source_files.is_empty());
}
