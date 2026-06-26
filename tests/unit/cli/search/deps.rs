#![allow(unused_imports)]

use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::cli::support::{
    normalize_temp_root, run_cli, run_search, run_search_with_stdin, write_manifest,
    write_search_fixture,
};

#[test]
fn cli_search_deps_reports_basic_manifest_topology_without_boundary() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"cli-search-deps-basic-usage\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [dependencies]\n\
         tokio = { version = \"1\", features = [\"rt\", \"time\", \"process\"] }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "use tokio::task;\n\npub async fn enqueue() {\n    task::spawn(async {});\n}\n",
    )
    .expect("write lib");

    let output = run_search(root, &["deps", "tokio"]);

    assert!(
        output.contains(
            "|dependency-topology dep=tokio usageLevel=manifest topology=asp-owned ownerUsage=0"
        ),
        "{output}"
    );
    assert!(
        output.contains(
            "source=manifest next=dependency-topology:tokio,crate-source:tokio,docs-use:tokio,import:tokio"
        ),
        "{output}"
    );
    assert!(!output.contains("|owner src/lib.rs"), "{output}");
}

#[test]
fn cli_search_deps_reports_manifest_topology_next_for_dependency() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"cli-search-deps-boundary-usage\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [dependencies]\n\
         tokio = { version = \"1\", features = [\"rt\", \"time\", \"process\"] }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "use std::time::Duration;\nuse tokio::time::timeout;\n\npub async fn bounded() {\n    let _ = timeout(Duration::from_secs(1), async {}).await;\n}\n",
    )
    .expect("write lib");

    let output = run_search(root, &["deps", "tokio"]);

    assert!(
        output.contains(
            "|dependency-topology dep=tokio usageLevel=manifest topology=asp-owned ownerUsage=0"
        ),
        "{output}"
    );
    assert!(
        output.contains(
            "source=manifest next=dependency-topology:tokio,crate-source:tokio,docs-use:tokio,import:tokio"
        ),
        "{output}"
    );
    assert!(!output.contains("|owner src/lib.rs"), "{output}");
}

#[test]
fn cli_search_deps_reports_no_output_before_external_dependency_lookup() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"cli-search-deps-missing-local\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [dependencies]\n\
         serde = { version = \"1\", features = [\"derive\"] }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "use serde::Serialize;\n#[derive(Serialize)]\npub struct Thing;\n",
    )
    .expect("write lib");

    let output = run_search(root, &["deps", "process-wrap::JobObject"]);

    assert!(
        output.starts_with(
            "[search-deps] q=process-wrap::JobObject pkg=. dep=0 own=0 api=0 apiQuery=JobObject"
        ),
        "{output}"
    );
    assert!(
        output.contains(
            "noOutput reason=no-local-dependency sourceTrace=cargo:manifest-empty,cargo:usage-empty"
        ),
        "{output}"
    );
    assert!(
        output.contains(
            "nextCommand=asp rust search deps 'process-wrap::JobObject' --workspace . --view seeds"
        ),
        "{output}"
    );
    assert!(
        output.contains("avoid=web-search,docs.rs-search,raw-read,external-doc-lookup"),
        "{output}"
    );
    assert!(!output.contains("docs-use"), "{output}");
    assert!(!output.contains("|owner src/lib.rs"), "{output}");

    let seeds = run_search(
        root,
        &["deps", "process-wrap::JobObject", "--view", "seeds"],
    );
    assert!(
        seeds.contains("D=dependency:pkg(process-wrap::JobObject)!dependency"),
        "{seeds}"
    );
    assert!(!seeds.contains("docs-use"), "{seeds}");

    let query_deps_seeds = run_search(
        root,
        &[
            "reasoning",
            "query-deps",
            "--query",
            "JobObject",
            "--dependency",
            "process-wrap",
            "--view",
            "seeds",
        ],
    );
    assert!(
        query_deps_seeds.starts_with("[search-reasoning] q=query-deps"),
        "{query_deps_seeds}"
    );
    assert!(
        query_deps_seeds.contains("process-wrap"),
        "{query_deps_seeds}"
    );
    assert!(
        query_deps_seeds.contains("avoid=web-search,docs.rs-search,raw-read"),
        "{query_deps_seeds}"
    );
    assert!(!query_deps_seeds.contains("docs-use"), "{query_deps_seeds}");

    let multi_word_query_deps_seeds = run_search(
        root,
        &[
            "reasoning",
            "query-deps",
            "--query",
            "Job Object",
            "--dependency",
            "process-wrap",
            "--view",
            "seeds",
        ],
    );
    assert!(
        multi_word_query_deps_seeds.contains("selector=query=Job%20Object"),
        "{multi_word_query_deps_seeds}"
    );
    assert!(
        multi_word_query_deps_seeds.contains("query=Job%20Object"),
        "{multi_word_query_deps_seeds}"
    );
    assert!(
        multi_word_query_deps_seeds.contains("process-wrap"),
        "{multi_word_query_deps_seeds}"
    );
    assert!(
        multi_word_query_deps_seeds.contains("avoid=web-search,docs.rs-search,raw-read"),
        "{multi_word_query_deps_seeds}"
    );
    assert!(
        !multi_word_query_deps_seeds.contains("docs-use"),
        "{multi_word_query_deps_seeds}"
    );
}

#[test]
fn cli_search_deps_distinguishes_external_version_queries() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"cli-search-deps-lock\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [dependencies]\n\
         process-wrap = \"8\"\n\
         serde = { version = \"1\", features = [\"derive\"] }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "use serde::de::DeserializeOwned;\nuse serde::Serialize;\n#[derive(Serialize)]\npub struct Thing;\npub fn decode<T: DeserializeOwned>() {}\n",
    )
    .expect("write lib");

    let query_deps_seeds = run_search(
        root,
        &[
            "reasoning",
            "query-deps",
            "--query",
            "Job Object",
            "--dependency",
            "process-wrap",
            "--view",
            "seeds",
        ],
    );
    assert!(
        query_deps_seeds.starts_with("[search-reasoning] q=query-deps"),
        "{query_deps_seeds}"
    );
    assert!(
        query_deps_seeds.contains("selector=query=Job"),
        "{query_deps_seeds}"
    );
    assert!(
        query_deps_seeds.contains("process-wrap"),
        "{query_deps_seeds}"
    );
    assert!(
        query_deps_seeds.contains("dependency"),
        "{query_deps_seeds}"
    );
    assert!(!query_deps_seeds.contains("docs-use"), "{query_deps_seeds}");
    assert!(
        query_deps_seeds.contains("crate-source"),
        "{query_deps_seeds}"
    );
    assert!(query_deps_seeds.contains("import"), "{query_deps_seeds}");
    assert!(query_deps_seeds.contains("tests"), "{query_deps_seeds}");
    assert!(query_deps_seeds.contains("frontier="), "{query_deps_seeds}");
    assert!(
        !query_deps_seeds.contains("D2=doc:path(process-wrap::Job Object)!docs"),
        "{query_deps_seeds}"
    );
    assert!(!query_deps_seeds.contains("D2.docs"), "{query_deps_seeds}");

    let current = run_search(root, &["deps", "serde@1"]);
    assert!(
        current.starts_with(
            "[search-deps] q=serde@1 pkg=. dep=1 own=0 api=0 requestedVersion=1 currentWorkspaceVersion=^1 versionScope=current"
        ),
        "{current}"
    );
    assert!(
        current.contains(
            "|dependency-topology dep=serde usageLevel=manifest topology=asp-owned ownerUsage=0"
        ),
        "{current}"
    );
    assert!(!current.contains("|owner src/lib.rs"), "{current}");

    let current_api = run_search(root, &["deps", "serde@1::Serialize"]);
    assert!(
        current_api.starts_with(
            "[search-deps] q=serde@1::Serialize pkg=. dep=1 own=1 api=0 requestedVersion=1 currentWorkspaceVersion=^1 versionScope=current apiQuery=Serialize"
        ),
        "{current_api}"
    );
    assert!(
        current_api.contains("|owner src/lib.rs hit_kind=dependency-api apiQuery=Serialize"),
        "{current_api}"
    );

    let current_subpath_api = run_search(root, &["deps", "serde/de@1::DeserializeOwned"]);
    assert!(
        current_subpath_api.starts_with(
            "[search-deps] q=serde/de@1::DeserializeOwned pkg=. dep=1 own=1 api=0 requestedVersion=1 currentWorkspaceVersion=^1 versionScope=current subpath=de apiQuery=DeserializeOwned"
        ),
        "{current_subpath_api}"
    );
    assert!(
        current_subpath_api.contains(
            "|owner src/lib.rs hit_kind=dependency-api subpath=de apiQuery=DeserializeOwned"
        ),
        "{current_subpath_api}"
    );
    assert!(
        current_subpath_api.contains(
            "|next dependency:serde,docs-use:serde/de::DeserializeOwned,crate-source:serde,import:serde,tests:DeserializeOwned"
        ),
        "{current_subpath_api}"
    );

    let external = run_search(root, &["deps", "serde@2::Serialize"]);

    assert!(
        external.starts_with(
            "[search-deps] q=serde@2::Serialize pkg=. dep=1 own=0 api=0 requestedVersion=2 currentWorkspaceVersion=^1 versionScope=external apiQuery=Serialize"
        ),
        "{external}"
    );
    assert!(
        external.contains("|dep serde import=serde pkg=serde version=^1 kind=normal opt=false source=manifest manager=cargo feat=derive"),
        "{external}"
    );
    assert!(
        external.contains("|note kind=version-scope message=requested-version-is-outside-current-workspace-version"),
        "{external}"
    );
    assert!(
        external.contains(
            "|next dependency:serde,docs-use:serde::Serialize,crate-source:serde,import:serde,tests:Serialize"
        ),
        "{external}"
    );
    assert!(
        external.contains("avoid=web-search,docs.rs-search,raw-read"),
        "{external}"
    );
    assert!(!external.contains("|owner src/lib.rs"), "{external}");

    let external_json = run_cli([
        "search".as_ref(),
        "deps".as_ref(),
        "serde@2::Serialize".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(external_json.status.success(), "{external_json:?}");
    let value = serde_json::from_slice::<Value>(&external_json.stdout).expect("external deps json");
    let header_fields = value["header"]["fields"]
        .as_object()
        .expect("header fields");
    assert_eq!(header_fields["requestedVersion"], "2");
    assert_eq!(header_fields["currentWorkspaceVersion"], "^1");
    assert_eq!(header_fields["versionScope"], "external");
    assert_eq!(header_fields["apiQuery"], "Serialize");
    assert!(!header_fields.contains_key("requested_version"));
    assert!(!header_fields.contains_key("version_scope"));
    assert!(!header_fields.contains_key("api_query"));
    let note_fields = value["notes"][0]["fields"]
        .as_object()
        .expect("note fields");
    assert_eq!(note_fields["kind"], "version-scope");
    assert_eq!(
        note_fields["message"],
        "requested-version-is-outside-current-workspace-version"
    );

    let external_subpath_api = run_search(root, &["deps", "serde/de@2::DeserializeOwned"]);
    assert!(
        external_subpath_api.starts_with(
            "[search-deps] q=serde/de@2::DeserializeOwned pkg=. dep=1 own=0 api=0 requestedVersion=2 currentWorkspaceVersion=^1 versionScope=external subpath=de apiQuery=DeserializeOwned"
        ),
        "{external_subpath_api}"
    );
    assert!(
        external_subpath_api.contains(
            "|note kind=version-scope message=requested-version-is-outside-current-workspace-version"
        ),
        "{external_subpath_api}"
    );
    assert!(
        external_subpath_api.contains(
            "|next dependency:serde,docs-use:serde/de::DeserializeOwned,crate-source:serde,import:serde,tests:DeserializeOwned"
        ),
        "{external_subpath_api}"
    );
    assert!(
        external_subpath_api.contains("avoid=web-search,docs.rs-search,raw-read"),
        "{external_subpath_api}"
    );
    assert!(
        !external_subpath_api.contains("|owner src/lib.rs"),
        "{external_subpath_api}"
    );
}
