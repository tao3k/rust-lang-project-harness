use serde_json::Value;
use tempfile::TempDir;

use super::support::{
    normalize_temp_root, run_cli, run_search, run_search_with_stdin, write_search_fixture,
};

#[test]
fn cli_rust_flow_sandbox_drill_exercises_registry_prime_search_and_ingest() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let registry = run_cli([
        "agent".as_ref(),
        "doctor".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(registry.status.success(), "{registry:?}");
    let registry_json = serde_json::from_slice::<Value>(&registry.stdout).expect("registry json");
    assert_eq!(
        registry_json["registryId"],
        "agent.semantic-protocols.semantic-language-registry"
    );
    assert_eq!(
        registry_json["protocolId"],
        "agent.semantic-protocols.semantic-language"
    );

    let language = registry_json["languages"][0].as_object().expect("language");
    assert_eq!(language["languageId"], "rust");
    assert_eq!(language["providerId"], "rs-harness");
    assert_eq!(language["binary"], "rs-harness");
    assert_eq!(
        language["namespace"],
        "agent.semantic-protocols.languages.rust.rs-harness"
    );
    let methods = language["methods"].as_array().expect("methods");
    for method in [
        "agent/doctor",
        "search/workspace",
        "search/prime",
        "search/dependency",
        "search/deps",
        "search/features",
        "search/ingest",
    ] {
        assert!(
            methods
                .iter()
                .any(|candidate| candidate.as_str() == Some(method)),
            "missing method {method}: {methods:?}"
        );
    }
    let schemas = language["schemas"].as_array().expect("schemas");
    for schema_id in [
        "agent.semantic-protocols.semantic-language-registry",
        "agent.semantic-protocols.semantic-search-packet",
    ] {
        assert!(
            schemas
                .iter()
                .any(|schema| schema["schemaId"].as_str() == Some(schema_id)),
            "missing schema {schema_id}: {schemas:?}"
        );
    }
    let method_descriptors = language["methodDescriptors"]
        .as_array()
        .expect("method descriptors");
    assert!(method_descriptors.iter().any(|descriptor| {
        descriptor["method"].as_str() == Some("search/ingest")
            && descriptor["acceptsStdin"].as_bool() == Some(true)
    }));

    let workspace = run_search(root, &["workspace"]);
    assert!(
        workspace.starts_with("[search-workspace] root=. pkg=1"),
        "{workspace}"
    );
    assert!(
        workspace.contains(
            "|package . root=. manifest=Cargo.toml source=manifest manager=cargo next=prime"
        ),
        "{workspace}"
    );

    let prime = run_search(root, &["prime"]);
    assert!(
        prime.starts_with("[search-prime] mode=package package=."),
        "{prime}"
    );
    assert!(
        prime.contains("|package . t=lib,test dep=anyhow,serde"),
        "{prime}"
    );
    assert!(prime.contains("|feature json"), "{prime}");
    assert!(
        prime.contains("|test-surface tests=tests next=tests"),
        "{prime}"
    );
    assert!(
        prime.contains(
            "|api-candidate Thing reason=public-item owner=src/domain/mod.rs next=docs:Thing"
        ),
        "{prime}"
    );
    assert!(prime.contains("|owner src/lib.rs"), "{prime}");
    assert!(prime.contains("|owner src/domain/mod.rs"), "{prime}");
    assert!(prime.contains("|next owner:src/lib.rs"), "{prime}");

    let dependency_trace = run_cli([
        "search".as_ref(),
        "dependency".as_ref(),
        "serde".as_ref(),
        "items".as_ref(),
        "public-api".as_ref(),
        "docs".as_ref(),
        "tests".as_ref(),
        "--trace".as_ref(),
        "--view".as_ref(),
        "both".as_ref(),
        root.as_os_str(),
    ]);
    assert!(dependency_trace.status.success(), "{dependency_trace:?}");
    let dependency_trace = normalize_temp_root(
        &String::from_utf8(dependency_trace.stdout).expect("dependency trace stdout"),
        root,
    );
    assert!(
        dependency_trace.starts_with(
            "[search-trace] source=dependency query=serde pipes=items,public-api,docs,tests view=both"
        ),
        "{dependency_trace}"
    );
    assert!(
        dependency_trace.contains("|stage dependency cargo=1 owners=2"),
        "{dependency_trace}"
    );
    assert!(
        dependency_trace.contains("|stage items owners=2 items="),
        "{dependency_trace}"
    );
    assert!(
        dependency_trace.contains("|stage public-api api=8"),
        "{dependency_trace}"
    );
    assert!(
        dependency_trace.contains("|stage docs docs=8"),
        "{dependency_trace}"
    );
    assert!(
        dependency_trace.contains("|stage tests tests=1"),
        "{dependency_trace}"
    );
    assert!(
        dependency_trace.contains("|stage output final=true"),
        "{dependency_trace}"
    );
    assert!(
        dependency_trace.contains("[search-dependency] q=serde pkg=. dep=1 own=2 api=8"),
        "{dependency_trace}"
    );
    assert!(
        dependency_trace.contains("|item load kind=fn"),
        "{dependency_trace}"
    );
    assert!(
        dependency_trace.contains("|api src/domain/mod.rs line=4 dep=serde kind=struct name=Thing"),
        "{dependency_trace}"
    );
    assert!(
        dependency_trace.contains("|test tests/domain.rs functions=1 owner=src/lib.rs"),
        "{dependency_trace}"
    );

    let feature = run_search(root, &["features", "json", "cfg", "owners", "tests"]);
    assert!(
        feature.starts_with("[search-features] q=json pkg=. feat=1 dep=1"),
        "{feature}"
    );
    assert!(feature.contains(" cfg=1"), "{feature}");
    assert!(feature.contains(" own=1"), "{feature}");
    assert!(
        feature.contains("|cfg feature:json declared_in=features expr=cfg(feature=\"json\")"),
        "{feature}"
    );
    assert!(
        feature.contains("|owner src/domain/mod.rs hit_kind=feature"),
        "{feature}"
    );

    let external_dep_api = run_search(root, &["deps", "serde@2::Serialize"]);
    assert!(
        external_dep_api.starts_with(
            "[search-deps] q=serde@2::Serialize pkg=. dep=1 own=0 api=0 requestedVersion=2 currentWorkspaceVersion=1 versionScope=external apiQuery=Serialize"
        ),
        "{external_dep_api}"
    );
    assert!(
        external_dep_api.contains(
            "|note kind=version-scope message=requested-version-is-outside-current-workspace-version"
        ),
        "{external_dep_api}"
    );
    assert!(
        external_dep_api.contains(
            "|next dependency:serde,docs:serde::Serialize,text:Serialize,tests:Serialize"
        ),
        "{external_dep_api}"
    );
    assert!(
        !external_dep_api.contains("|owner src/lib.rs"),
        "{external_dep_api}"
    );

    let ingest = run_search_with_stdin(root, &["ingest"], "src/lib.rs:6:pub fn load() -> Thing\n");
    assert!(
        ingest.starts_with("[search-ingest] src=rg-n in=1 own=1"),
        "{ingest}"
    );
    assert!(
        ingest.contains("|owner src/lib.rs role=source hit_kind=text locations=6:1 next=owner"),
        "{ingest}"
    );
}
