#[test]
fn rust_tree_sitter_queries_follow_upstream_layout() {
    let provider_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tree-sitter")
        .join("tree-sitter-rust");
    assert!(provider_root.join("corpus-profile.json").is_file());
    assert!(
        std::fs::read_dir(&provider_root)
            .expect("provider tree-sitter-rust dir")
            .filter_map(Result::ok)
            .all(|entry| entry.path().extension().and_then(|ext| ext.to_str()) != Some("scm")),
        "Rust tree-sitter queries must live under queries/"
    );

    let queries_root = provider_root.join("queries");
    for name in [
        "calls.scm",
        "cfg.scm",
        "declarations.scm",
        "imports.scm",
        "injections.scm",
        "macros.scm",
        "tags.scm",
    ] {
        assert!(queries_root.join(name).is_file(), "missing {name}");
    }
}
