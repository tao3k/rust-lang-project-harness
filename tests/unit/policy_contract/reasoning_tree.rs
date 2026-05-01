use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn rust_reasoning_tree_facts_are_the_policy_facing_parser_layer() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parser_source = fs::read_to_string(root.join("src/parser/reasoning_tree.rs"))
        .expect("read reasoning tree parser");
    assert!(parser_source.contains("pub(crate) fn rust_reasoning_tree_facts"));
    assert!(parser_source.contains("RustReasoningTreeFacts"));
    assert!(parser_source.contains("declared_child_edges"));
    assert!(parser_source.contains("import_summary"));
    assert!(parser_source.contains("local_owner_imports"));
    assert!(parser_source.contains("RustReasoningOwnerDependencyFacts"));
    assert!(parser_source.contains("owner_branches"));
    assert!(parser_source.contains("RustReasoningOwnerBranchFacts"));
    assert!(
        fs::read_to_string(root.join("src/parser/module_tree.rs"))
            .expect("read module tree parser")
            .contains("RustModuleChildEdgeKind")
    );

    let forbidden_rule_fragments = [
        "rust_module_tree_facts(",
        "rust_source_path_facts(",
        "RustModuleTreeFacts",
        "RustSourcePathFacts",
    ];
    let mut offenders = Vec::new();
    for path in rust_files_under(&root.join("src/rules")) {
        let source = fs::read_to_string(&path).expect("read rule source");
        if forbidden_rule_fragments
            .iter()
            .any(|fragment| source.contains(fragment))
        {
            offenders.push(relative_path(&root, &path));
        }
    }

    assert!(
        offenders.is_empty(),
        "Policy rules must consume parser reasoning-tree facts instead of lower-level parser path or module-tree facts: {offenders:?}"
    );
}

fn rust_files_under(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust_files(root, &mut files);
    files.sort();
    files
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read directory") {
        let path = entry.expect("read directory entry").path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("path under repository root")
        .to_string_lossy()
        .replace('\\', "/")
}
