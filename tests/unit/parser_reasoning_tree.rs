use std::fs;

use tempfile::TempDir;

use crate::RustProjectHarnessScope;
use crate::parser::{
    RustModuleChildEdgeKind, RustReasoningOwnerBranchRole, parse_rust_file,
    rust_reasoning_tree_facts,
};

#[test]
fn reasoning_tree_interprets_modules_owners_and_child_edges() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let src = root.join("src");
    fs::create_dir_all(src.join("domain")).expect("create source tree");
    fs::create_dir_all(src.join("alt")).expect("create alternate source tree");
    fs::write(
        src.join("lib.rs"),
        "//! Crate facade.\nuse crate::domain::Thing;\nuse std::fmt;\nmod domain;\n#[path = \"alt/custom.rs\"]\nmod custom;\ninclude!(\"shard.rs\");\n",
    )
    .expect("write lib");
    fs::write(
        src.join("domain.rs"),
        "//! Domain branch.\nuse self::leaf::Leaf;\nuse super::sibling::Thing;\nmod leaf;\n",
    )
    .expect("write domain");
    fs::write(src.join("domain/leaf.rs"), "//! Domain leaf.\n").expect("write leaf");
    fs::write(src.join("alt/custom.rs"), "//! Custom path child.\n").expect("write custom");
    fs::write(src.join("shard.rs"), "//! Included shard.\n").expect("write shard");

    let modules = [
        parse_rust_file(&src.join("lib.rs")),
        parse_rust_file(&src.join("domain.rs")),
        parse_rust_file(&src.join("domain/leaf.rs")),
        parse_rust_file(&src.join("alt/custom.rs")),
        parse_rust_file(&src.join("shard.rs")),
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
    assert_eq!(lib.import_summary.crate_imports, 1);
    assert_eq!(lib.import_summary.external_imports, 1);
    assert_eq!(
        child_edges(lib),
        vec![
            (RustModuleChildEdgeKind::Mod, src.join("domain.rs")),
            (
                RustModuleChildEdgeKind::PathAttrMod,
                src.join("alt/custom.rs")
            ),
            (
                RustModuleChildEdgeKind::IncludeLiteral,
                src.join("shard.rs")
            ),
        ]
    );

    let branch = reasoning_tree
        .module(&src.join("domain.rs"))
        .expect("branch facts");
    assert_eq!(
        branch.source_path.namespace_components,
        vec!["src".to_string(), "domain".to_string()]
    );
    assert_eq!(branch.import_summary.self_imports, 1);
    assert_eq!(branch.import_summary.parent_imports, 1);
    assert_eq!(
        child_edges(branch),
        vec![(RustModuleChildEdgeKind::Mod, src.join("domain/leaf.rs"))]
    );

    assert_eq!(reasoning_tree.owner_branches.len(), 2);
    assert_eq!(reasoning_tree.owner_branches[0].path, src.join("lib.rs"));
    assert_eq!(
        reasoning_tree.owner_branches[0].roles,
        vec![
            RustReasoningOwnerBranchRole::Root,
            RustReasoningOwnerBranchRole::Facade,
        ]
    );
    assert_eq!(
        reasoning_tree.owner_branches[0].owner_namespace,
        vec!["src".to_string()]
    );
    assert_eq!(
        owner_branch_child_edges(&reasoning_tree.owner_branches[0]),
        vec![
            (RustModuleChildEdgeKind::Mod, src.join("domain.rs")),
            (
                RustModuleChildEdgeKind::PathAttrMod,
                src.join("alt/custom.rs")
            ),
            (
                RustModuleChildEdgeKind::IncludeLiteral,
                src.join("shard.rs")
            ),
        ]
    );
    assert_eq!(
        reasoning_tree.owner_branches[0]
            .import_summary
            .crate_imports,
        1
    );
    assert_eq!(
        reasoning_tree.owner_branches[0]
            .import_summary
            .external_imports,
        1
    );
    assert_eq!(reasoning_tree.owner_branches[1].path, src.join("domain.rs"));
    assert_eq!(
        reasoning_tree.owner_branches[1].roles,
        vec![RustReasoningOwnerBranchRole::Branch]
    );
    assert_eq!(
        reasoning_tree.owner_branches[1].owner_namespace,
        vec!["src".to_string(), "domain".to_string()]
    );
    assert_eq!(
        reasoning_tree.owner_branches[1].import_summary.self_imports,
        1
    );
    assert_eq!(
        reasoning_tree.owner_branches[1]
            .import_summary
            .parent_imports,
        1
    );

    assert!(reasoning_tree.shadowed_module_sources.is_empty());
    assert!(reasoning_tree.unreachable_source_files.is_empty());
}

#[test]
fn reasoning_tree_marks_test_root_modules_as_test_sources() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let src = root.join("src");
    let tests = root.join("tests");
    fs::create_dir_all(&src).expect("create source tree");
    fs::create_dir_all(&tests).expect("create tests tree");
    fs::write(src.join("lib.rs"), "//! Crate facade.\n").expect("write lib");
    fs::write(tests.join("integration.rs"), "use crate::prelude::*;\n").expect("write integration");

    let modules = [
        parse_rust_file(&src.join("lib.rs")),
        parse_rust_file(&tests.join("integration.rs")),
    ];
    let scope = RustProjectHarnessScope {
        project_root: root.to_path_buf(),
        source_paths: vec![src],
        test_paths: vec![tests.clone()],
        package_paths: Vec::new(),
        fallback_paths: vec![root.to_path_buf()],
    };

    let reasoning_tree = rust_reasoning_tree_facts(&scope, &modules);
    let test_module = reasoning_tree
        .module(&tests.join("integration.rs"))
        .expect("test module facts");

    assert!(test_module.source_path.is_test_source);
    assert!(!test_module.is_source_module);
    assert!(
        !reasoning_tree
            .owner_branches
            .iter()
            .any(|branch| branch.path.ends_with("tests/integration.rs"))
    );
}

fn child_edges(
    module: &crate::parser::RustReasoningModuleFacts,
) -> Vec<(RustModuleChildEdgeKind, std::path::PathBuf)> {
    module
        .declared_child_edges
        .iter()
        .map(|edge| (edge.kind, edge.child_path.clone()))
        .collect()
}

fn owner_branch_child_edges(
    branch: &crate::parser::RustReasoningOwnerBranchFacts,
) -> Vec<(RustModuleChildEdgeKind, std::path::PathBuf)> {
    branch
        .declared_child_edges
        .iter()
        .map(|edge| (edge.kind, edge.child_path.clone()))
        .collect()
}
