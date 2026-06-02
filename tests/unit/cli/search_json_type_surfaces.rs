use serde_json::Value;
use tempfile::TempDir;

use super::support::{run_cli, write_search_fixture};

#[test]
fn cli_search_public_external_types_json_exposes_type_surfaces() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let json = run_cli([
        "search".as_ref(),
        "public-external-types".as_ref(),
        "--dependency".as_ref(),
        "serde".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(json.status.success(), "{json:?}");
    let stdout = String::from_utf8(json.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("search json");

    let hits = value["hits"].as_array().expect("hits");
    assert!(hits.iter().any(|hit| {
        hit["kind"] == "external-type"
            && hit["ownerPath"] == "src/lib.rs"
            && hit["location"]["path"] == "src/lib.rs"
            && hit["location"]["line"].as_u64().is_some()
    }));

    let type_surfaces = value["typeSurfaces"].as_array().expect("type surfaces");
    assert!(type_surfaces.iter().any(|surface| {
        surface["role"] == "api-field"
            && surface["ownerPath"] == "src/lib.rs"
            && surface["package"] == "serde"
            && surface["carrier"]["name"] == "serde::Serialize"
            && surface["carrier"]["carrier"] == "external"
            && surface["carrier"]["external"] == true
            && surface["fields"]["dependency"] == "serde"
    }));
}
