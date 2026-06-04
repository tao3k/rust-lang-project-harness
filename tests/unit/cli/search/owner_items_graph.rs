use tempfile::TempDir;

use crate::cli::support::{run_search, write_search_fixture};

#[test]
fn cli_search_owner_items_graph_prioritizes_symbol_code_frontier() {
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
            "[search-owner] owner=src/domain/mod.rs selector=items terms=1 view=seeds alg=seed-frontier"
        ),
        "{output}"
    );
    assert!(
        output.contains("alias: graph:{G=search,O=owner,S=symbol}"),
        "{output}"
    );
    assert!(
        output.contains(
            "O=owner:path(src/domain/mod.rs)!owner;S=symbol:symbol(Thing)@src/domain/mod.rs:4:5!code"
        ),
        "{output}"
    );
    assert!(output.contains("G>{O:selects,S:matches}"), "{output}");
    assert!(output.contains("O>{S:contains}"), "{output}");
    assert!(output.contains("rank=S,O frontier=S.code"), "{output}");
    assert!(
        output.contains("omit=code,comments,blank-lines,nonmatching-items"),
        "{output}"
    );
    assert!(
        output.contains("avoid=repeat-owner,raw-read,full-json"),
        "{output}"
    );
    assert!(!output.contains("S.symbol"), "{output}");
    assert!(!output.contains("frontier=O.owner"), "{output}");
}
