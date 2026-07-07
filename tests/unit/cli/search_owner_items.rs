use super::support::{run_search, write_search_fixture};

#[test]
fn owner_items_inventory_omits_flow_lines() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    write_search_fixture(root);

    let rendered = run_search(root, &["owner", "src/lib.rs", "items"]);

    assert!(
        rendered.starts_with("[search-owner] q=src/lib.rs"),
        "{rendered}"
    );
    assert!(rendered.contains(" item="), "{rendered}");
    assert!(rendered.contains("|item "), "{rendered}");
    assert!(!rendered.contains("|code "), "{rendered}");
    assert!(!rendered.contains("|test "), "{rendered}");
    assert!(!rendered.contains("|edge "), "{rendered}");
    assert!(!rendered.contains("|synthesis "), "{rendered}");
    assert!(!rendered.contains("|next "), "{rendered}");
}

#[test]
fn owner_item_query_seeds_render_code_frontier() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    write_search_fixture(root);

    let rendered = run_search(
        root,
        &[
            "owner",
            "src/lib.rs",
            "items",
            "--query",
            "load",
            "--view",
            "seeds",
        ],
    );

    assert!(
        rendered.starts_with("[search]") || rendered.starts_with("[search-owner]"),
        "{rendered}"
    );
    assert!(
        rendered.contains("I=item:symbol(load)@src/lib.rs:")
            || rendered.contains("N=syntax:target(load)!syntax")
            || rendered.contains("O=owner:path(src/lib.rs)!owner"),
        "{rendered}"
    );
    assert!(
        rendered.contains("syntax I selector=src/lib.rs:")
            || rendered.contains("O=owner:path(src/lib.rs)!owner"),
        "{rendered}"
    );
    assert!(
        rendered.contains("frontier=I.syntax")
            || rendered.contains("frontier=O.owner,N.syntax")
            || rendered.contains("frontier=O.owner"),
        "{rendered}"
    );
    assert!(!rendered.contains("fn load"), "{rendered}");
}
