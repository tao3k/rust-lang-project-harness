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
