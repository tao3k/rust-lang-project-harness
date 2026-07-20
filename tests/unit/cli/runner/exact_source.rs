use super::{ExactSelector, module_owner_candidates};

#[test]
fn exact_selector_rejects_workspace_escape() {
    let error = ExactSelector::parse("rust://../outside.rs#item/struct/Secret")
        .err()
        .expect("selector must be rejected");

    assert!(error.contains("escapes workspace"));
}

#[test]
fn external_module_candidates_follow_rust_module_layout() {
    let module: syn::ItemMod = syn::parse_quote!(
        mod dispatch;
    );

    assert_eq!(
        module_owner_candidates("src/cli/runner/mod.rs", &module),
        [
            "src/cli/runner/dispatch.rs".to_string(),
            "src/cli/runner/dispatch/mod.rs".to_string(),
        ]
    );
}

fn pinned_workspace_with_sources(
    case: &str,
    sources: &[(&str, &str)],
) -> (std::path::PathBuf, super::PinnedWorkspace) {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "rs-harness-exact-source-{case}-{}-{nonce}",
        std::process::id()
    ));
    for (path, source) in sources {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("source path has parent"))
            .expect("create source parent");
        std::fs::write(path, source).expect("write source fixture");
    }
    let pinned = super::PinnedWorkspace::capture(&root).expect("capture pinned workspace");
    (root, pinned)
}

#[test]
fn stale_exact_selector_relocates_to_one_snapshot_owner() {
    let (root, pinned) = pinned_workspace_with_sources(
        "relocated",
        &[
            ("src/cli/runner/mod.rs", "mod options;\n"),
            (
                "src/cli/runner/dispatch.rs",
                "pub(super) struct AgentOptions { json: bool }\n",
            ),
        ],
    );
    let selector = ExactSelector::parse("rust://src/cli/runner/mod.rs#item/struct/AgentOptions")
        .expect("selector parses");
    assert!(
        super::resolve_live_item(&pinned, &selector)
            .expect("probe requested owner")
            .is_none(),
        "requested owner must be stale for this fixture"
    );

    let super::RelocationOutcome::Resolved(resolved) =
        super::relocate_live_item(&pinned, &selector).expect("relocate item")
    else {
        panic!("expected one relocated item");
    };
    assert_eq!(resolved.owner_path, "src/cli/runner/dispatch.rs");
    assert!(resolved.code.contains("struct AgentOptions"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn stale_exact_selector_reports_sorted_ambiguity() {
    let (root, pinned) = pinned_workspace_with_sources(
        "ambiguous",
        &[
            ("src/a.rs", "pub struct AgentOptions;\n"),
            ("src/b.rs", "pub struct AgentOptions;\n"),
        ],
    );
    let selector = ExactSelector::parse("rust://src/old.rs#item/struct/AgentOptions")
        .expect("selector parses");

    let super::RelocationOutcome::Ambiguous(candidates) =
        super::relocate_live_item(&pinned, &selector).expect("collect candidates")
    else {
        panic!("expected ambiguous relocation");
    };
    assert_eq!(
        candidates,
        [
            "rust://src/a.rs#item/struct/AgentOptions",
            "rust://src/b.rs#item/struct/AgentOptions",
        ]
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn stale_exact_selector_distinguishes_kind_mismatch_from_absence() {
    let (root, pinned) = pinned_workspace_with_sources(
        "kind-mismatch",
        &[("src/current.rs", "pub enum AgentOptions { Json }\n")],
    );
    let selector = ExactSelector::parse("rust://src/old.rs#item/struct/AgentOptions")
        .expect("selector parses");
    let super::RelocationOutcome::KindMismatch(actual_kinds) =
        super::relocate_live_item(&pinned, &selector).expect("classify kind mismatch")
    else {
        panic!("expected item kind mismatch");
    };
    assert_eq!(actual_kinds, ["enum"]);

    let absent = ExactSelector::parse("rust://src/old.rs#item/struct/MissingOptions")
        .expect("absent selector parses");
    assert!(matches!(
        super::relocate_live_item(&pinned, &absent).expect("classify absence"),
        super::RelocationOutcome::Missing
    ));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn inline_module_items_are_resolved_by_the_live_parser() {
    let source = "mod options { pub struct AgentOptions; }";
    let syntax = crate::parser::parse_rust_source_syntax(source).expect("parse inline module");
    let mut items = Vec::new();
    super::collect_parse_artifact_items(source, &syntax.items, &mut items);

    assert!(
        items
            .iter()
            .any(|item| item.kind == "struct" && item.name == "AgentOptions")
    );
}
