use tempfile::TempDir;

use crate::cli::support::{run_search, write_search_fixture};

#[test]
fn cli_search_owner_items_graph_prioritizes_symbol_code_frontier() {
    if crate::cli::support::skip_if_protocol_graph_renderer_unavailable() {
        return;
    }

    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let output = run_search(
        root,
        &[
            "owner",
            "src/domain/mod.rs",
            "items",
            "--query",
            "Thing",
            "--view",
            "seeds",
        ],
    );

    assert!(
        output.contains("O=owner:path(src/domain/mod.rs)!owner"),
        "{output}"
    );
    assert!(
        output.contains("aliases: graph:{G=search,O=owner}"),
        "{output}"
    );
    assert!(output.contains("G>{O:selects}"), "{output}");
    assert!(output.contains("rank=O frontier=O.owner"), "{output}");
    assert!(!output.contains("S=symbol"), "{output}");
}
