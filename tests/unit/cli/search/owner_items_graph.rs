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
        output.starts_with(
            "[search-owner] q=src/domain/mod.rs pkg=. selector=items querySet=1 alg=item-frontier"
        ),
        "{output}"
    );
    assert!(
        output.contains("aliases: graph:{G=search,O=owner,Q=query,I=item}"),
        "{output}"
    );
    assert!(
        output.contains(
            "O=owner:path(src/domain/mod.rs)!owner;Q=query:term(Thing)!query;I=item:symbol(Thing)@src/domain/mod.rs:4:5!code"
        ),
        "{output}"
    );
    assert!(output.contains("G>{O:selects,Q:matches}"), "{output}");
    assert!(output.contains("O>{I:contains}"), "{output}");
    assert!(output.contains("Q>{I:matches}"), "{output}");
    assert!(output.contains("rank=I,O frontier=I.code"), "{output}");
    assert!(
        output.contains("omit=code,projection-nodes,large-item-text"),
        "{output}"
    );
    assert!(
        output.contains("avoid=inline-code-in-search,raw-read,repeat-owner"),
        "{output}"
    );
    assert!(!output.contains("S=symbol"), "{output}");
    assert!(!output.contains("frontier=O.owner"), "{output}");
}
