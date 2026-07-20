use tempfile::TempDir;

use crate::cli::support::run_search;

#[test]
fn cli_search_api_discovers_public_method_without_explicit_return_type() {
    if crate::cli::support::skip_if_protocol_graph_renderer_unavailable() {
        return;
    }

    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("create src");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"callable-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub struct Service;\nimpl Service {\n    pub fn ping(&self) {}\n}\n",
    )
    .expect("write source");

    let api = run_search(root, &["api", "ping"]);
    assert!(
        api.starts_with("[search-api] q=ping pkg=. api=1 source=native-parser"),
        "{api}"
    );
    assert!(
        api.contains(
            "kind=method name=ping next=owner:src/lib.rs source=native-parser apiKind=method public=true docs=false"
        ),
        "{api}"
    );
}
