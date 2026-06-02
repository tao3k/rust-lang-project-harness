use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use super::support::{
    normalize_temp_root, run_cli, run_search, run_search_with_stdin, write_manifest,
    write_search_fixture,
};

#[test]
fn cli_search_prime_renders_line_protocol() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-search-prime");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod domain;\nuse crate::domain::Thing;\npub fn load() -> Thing { Thing }\n",
    )
    .expect("write lib");
    fs::write(root.join("src/hook_runtime.rs"), "pub fn execute() {}\n").expect("write path owner");
    fs::write(
        root.join("src/domain/mod.rs"),
        "//! Domain branch.\npub struct Thing;\n",
    )
    .expect("write domain");

    let output = run_cli(["search".as_ref(), "prime".as_ref(), root.as_os_str()]);

    assert!(output.status.success(), "{output:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(
        stdout.starts_with("[search-prime] mode=package package=."),
        "{stdout}"
    );
    assert!(stdout.contains("|package ."), "{stdout}");
    assert!(stdout.contains("|owner src/lib.rs"), "{stdout}");
    assert!(
        stdout.contains("|edge O:src/lib.rs -mod-> O:src/domain/mod.rs"),
        "{stdout}"
    );
    assert!(
        stdout.contains("|edge O:src/lib.rs -crate:crate-> O:src/domain/mod.rs"),
        "{stdout}"
    );
    assert!(stdout.contains("|next owner:src/lib.rs"), "{stdout}");
    assert!(!stdout.contains("Modules:"), "{stdout}");
    assert!(!stdout.trim_start().starts_with('{'), "{stdout}");
    insta::assert_snapshot!("cli_search_prime", stdout);
}

#[test]
fn cli_search_json_and_trace_follow_rfc_output_modes() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let json = run_cli([
        "search".as_ref(),
        "prime".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(json.status.success(), "{json:?}");
    let stdout = String::from_utf8(json.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("search json");
    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-search-packet"
    );
    assert_eq!(value["schemaVersion"], "1");
    assert_eq!(
        value["protocolId"],
        "agent.semantic-protocols.semantic-language"
    );
    assert_eq!(value["protocolVersion"], "1");
    assert_eq!(value["languageId"], "rust");
    assert_eq!(value["providerId"], "rs-harness");
    assert_eq!(value["binary"], "rs-harness");
    assert_eq!(
        value["namespace"],
        "agent.semantic-protocols.semantic-language"
    );
    assert_eq!(value["method"], "search/prime");
    assert_eq!(value["view"], "prime");
    assert_eq!(value["renderMode"], "graph");
    assert_eq!(value["header"]["kind"], "search-prime");
    assert!(value["packages"].as_array().expect("packages").len() == 1);
    assert!(value["owners"].as_array().expect("owners").len() > 1);
    assert!(!value["edges"].as_array().expect("edges").is_empty());
    assert_eq!(value["searchSynthesis"]["algorithm"], "owner-rank-frontier");
    assert_eq!(value["searchSynthesis"]["scope"], "prime");
    assert!(
        value["searchSynthesis"]["highImpactOwners"]
            .as_array()
            .expect("high impact owners")
            .iter()
            .any(|path| path.as_str() == Some("src/lib.rs")),
        "{value}"
    );
    assert!(value.get("compact").is_none(), "{value}");

    let trace = run_cli([
        "search".as_ref(),
        "dependency".as_ref(),
        "serde".as_ref(),
        "items".as_ref(),
        "--trace".as_ref(),
        "--view".as_ref(),
        "both".as_ref(),
        root.as_os_str(),
    ]);
    assert!(trace.status.success(), "{trace:?}");
    let stdout = String::from_utf8(trace.stdout).expect("utf8 stdout");
    assert!(
        stdout.starts_with("[search-trace] source=dependency query=serde pipes=items view=both"),
        "{stdout}"
    );
    assert!(
        stdout.contains("|stage cargo=1 owners=2 items="),
        "{stdout}"
    );
    assert!(stdout.contains(" final=true lines="), "{stdout}");
    assert!(stdout.contains("[search-dependency] q=serde"), "{stdout}");
}

#[test]
fn cli_search_cargo_namespace_is_not_a_compatibility_alias() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-search-no-cargo-alias");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let output = run_cli(["search".as_ref(), "cargo".as_ref(), root.as_os_str()]);

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("unknown search view: cargo"), "{stderr}");
}

#[test]
fn cli_search_cfg_reads_manifest_facts() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"cli-search-cfg\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [features]\n\
         json = []\n\n\
         [lints.rust]\n\
         unexpected_cfgs = { level = \"warn\", check-cfg = ['cfg(loom)'] }\n\n\
         [target.'cfg(loom)'.dev-dependencies]\n\
         loom = { version = \"0.7\", features = [\"futures\"] }\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let loom = run_search(root, &["cfg", "loom"]);
    assert!(
        loom.starts_with("[search-cfg] q=loom pkg=. cfg=2 dep=1 own=0"),
        "{loom}"
    );
    assert!(
        loom.contains("|cfg loom declared_in=lints.rust.unexpected_cfgs expr=cfg(loom) source=manifest manager=cargo"),
        "{loom}"
    );
    assert!(
        loom.contains(
            "|cfg loom declared_in=target.dependencies expr=cfg(loom) source=manifest manager=cargo"
        ),
        "{loom}"
    );
    assert!(
        loom.contains(
            "|dep loom import=loom pkg=loom version=0.7 kind=dev opt=false source=manifest manager=cargo target=cfg(loom) feat=futures"
        ),
        "{loom}"
    );
    assert!(
        loom.contains("|next text:cfg(loom)(scope=src),text:loom(scope=tests)"),
        "{loom}"
    );

    let feature = run_search(root, &["cfg", "json"]);
    assert!(
        feature.starts_with("[search-cfg] q=json pkg=. cfg=1 dep=0 own=0"),
        "{feature}"
    );
    assert!(
        feature.contains("|cfg feature:json declared_in=features expr=cfg(feature=\"json\") source=manifest manager=cargo"),
        "{feature}"
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
         serde = { version = \"1\", features = [\"derive\"] }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "use serde::de::DeserializeOwned;\nuse serde::Serialize;\n#[derive(Serialize)]\npub struct Thing;\npub fn decode<T: DeserializeOwned>() {}\n",
    )
    .expect("write lib");

    let current = run_search(root, &["deps", "serde@1"]);
    assert!(
        current.starts_with(
            "[search-deps] q=serde@1 pkg=. dep=1 own=1 api=0 requestedVersion=1 currentWorkspaceVersion=1 versionScope=current"
        ),
        "{current}"
    );
    assert!(
        current.contains("|owner src/lib.rs hit_kind=dependency"),
        "{current}"
    );

    let current_api = run_search(root, &["deps", "serde@1::Serialize"]);
    assert!(
        current_api.starts_with(
            "[search-deps] q=serde@1::Serialize pkg=. dep=1 own=1 api=0 requestedVersion=1 currentWorkspaceVersion=1 versionScope=current apiQuery=Serialize"
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
            "[search-deps] q=serde/de@1::DeserializeOwned pkg=. dep=1 own=1 api=0 requestedVersion=1 currentWorkspaceVersion=1 versionScope=current subpath=de apiQuery=DeserializeOwned"
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
        current_subpath_api.contains("|next dependency:serde,docs:serde/de::DeserializeOwned"),
        "{current_subpath_api}"
    );

    let external = run_search(root, &["deps", "serde@2::Serialize"]);

    assert!(
        external.starts_with(
            "[search-deps] q=serde@2::Serialize pkg=. dep=1 own=0 api=0 requestedVersion=2 currentWorkspaceVersion=1 versionScope=external apiQuery=Serialize"
        ),
        "{external}"
    );
    assert!(
        external.contains("|dep serde import=serde pkg=serde version=1 kind=normal opt=false source=manifest manager=cargo feat=derive"),
        "{external}"
    );
    assert!(
        external.contains("|note kind=version-scope message=requested-version-is-outside-current-workspace-version"),
        "{external}"
    );
    assert!(
        external.contains(
            "|next dependency:serde,docs:serde::Serialize,text:Serialize,tests:Serialize"
        ),
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
    assert_eq!(header_fields["currentWorkspaceVersion"], "1");
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
            "[search-deps] q=serde/de@2::DeserializeOwned pkg=. dep=1 own=0 api=0 requestedVersion=2 currentWorkspaceVersion=1 versionScope=external subpath=de apiQuery=DeserializeOwned"
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
        external_subpath_api.contains("|next dependency:serde,docs:serde/de::DeserializeOwned"),
        "{external_subpath_api}"
    );
    assert!(
        !external_subpath_api.contains("|owner src/lib.rs"),
        "{external_subpath_api}"
    );
}

#[test]
fn cli_search_fzf_renders_fuzzy_frontier() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"cli-search-fzf\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "pub mod hook_runtime;\npub struct AgentHookEvent;\npub fn run_codex_agent_hook(_event: AgentHookEvent) {}\npub fn source_snapshot() { assert_snapshot!(\"src\"); }\n",
    )
    .expect("write lib");
    fs::create_dir_all(root.join("tests/unit")).expect("create tests unit");
    fs::write(
        root.join("tests/unit/snapshot.rs"),
        "fn snapshot_case() { assert_snapshot!(\"ok\"); }\n",
    )
    .expect("write scoped test");

    let fzf = run_search(root, &["fzf", "runCodexAgentHook"]);
    assert!(
        fzf.starts_with("[search-fzf] q=runCodexAgentHook mode=fuzzy backend=provider pkg=. own=1"),
        "{fzf}"
    );
    assert!(fzf.contains("|owner src/lib.rs hit_kind=fzf"), "{fzf}");
    let path_fzf = run_search(root, &["fzf", "src/lib.rs", "--view", "seeds"]);
    assert!(path_fzf.contains("owner:src/lib.rs"), "{path_fzf}");
    let scoped_fzf = run_search(
        root,
        &[
            "fzf",
            "assert_snapshot!",
            "owner",
            "tests/unit",
            "--view",
            "seeds",
        ],
    );
    assert!(scoped_fzf.contains("[search-graph]"), "{scoped_fzf}");
    assert!(
        scoped_fzf.contains("owner:tests/unit/snapshot.rs!owner"),
        "{scoped_fzf}"
    );
    assert!(scoped_fzf.contains("frontier="), "{scoped_fzf}");
    assert!(!scoped_fzf.contains("|seed "), "{scoped_fzf}");
    assert!(!scoped_fzf.contains("src/lib.rs"), "{scoped_fzf}");

    let multi_scoped_fzf = run_search(
        root,
        &[
            "fzf",
            "assert_snapshot!",
            "owner",
            "src",
            "tests/unit",
            "--view",
            "seeds",
        ],
    );
    assert!(
        multi_scoped_fzf.contains("src/lib.rs"),
        "{multi_scoped_fzf}"
    );
    assert!(
        multi_scoped_fzf.contains("owner:tests/unit/snapshot.rs"),
        "{multi_scoped_fzf}"
    );

    let cwd_discovered_output = Command::new(env!("CARGO_BIN_EXE_rs-harness"))
        .current_dir(root)
        .args([
            "search",
            "fzf",
            "assert_snapshot!",
            "owner",
            "src",
            "tests/unit",
            "--view",
            "seeds",
        ])
        .output()
        .expect("run cli");
    assert!(
        cwd_discovered_output.status.success(),
        "{cwd_discovered_output:?}"
    );
    let cwd_discovered_fzf = normalize_temp_root(
        &String::from_utf8(cwd_discovered_output.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(
        cwd_discovered_fzf.contains("src/lib.rs"),
        "{cwd_discovered_fzf}"
    );
    assert!(
        cwd_discovered_fzf.contains("owner:tests/unit/snapshot.rs"),
        "{cwd_discovered_fzf}"
    );

    let query_set = run_search(
        root,
        &[
            "fzf",
            "--query-set",
            "AgentHookEvent",
            "--query-set",
            "runCodexAgentHook",
            "--view",
            "seeds",
        ],
    );
    assert!(
        query_set.starts_with(
            "[search-fzf] q=AgentHookEvent,runCodexAgentHook querySet=2 selector=fuzzy-set mode=fuzzy backend=provider pkg=. own=1"
        ),
        "{query_set}"
    );
    assert!(
        query_set.contains("[search-graph] mode=query-set"),
        "{query_set}"
    );
    assert!(query_set.contains("owner:src/lib.rs!owner"), "{query_set}");
    assert!(query_set.contains("frontier="), "{query_set}");
    assert!(!query_set.contains("|seed "), "{query_set}");

    let fuzzy_acronym = run_search(root, &["fzf", "rCAH", "--view", "seeds"]);
    assert!(
        fuzzy_acronym.contains("owner:src/lib.rs!owner"),
        "{fuzzy_acronym}"
    );
    assert!(fuzzy_acronym.contains("frontier="), "{fuzzy_acronym}");
    assert!(!fuzzy_acronym.contains("|seed "), "{fuzzy_acronym}");
    let exact_acronym = run_search(
        root,
        &["fzf", "rCAH", "--view", "seeds", "--fzf-arg", "--exact"],
    );
    assert!(
        exact_acronym.starts_with(
            "[search-fzf] q=rCAH mode=exact backend=provider pkg=. own=0 finder=fzf fzfArgs=--exact"
        ),
        "{exact_acronym}"
    );
    assert!(
        !exact_acronym.contains("owner:src/lib.rs!owner"),
        "{exact_acronym}"
    );
    assert!(!exact_acronym.contains("|seed "), "{exact_acronym}");
    let boundary_exact = run_cli([
        "search".as_ref(),
        "fzf".as_ref(),
        "rCAH".as_ref(),
        "--view".as_ref(),
        "seeds".as_ref(),
        root.as_os_str(),
        "--fzf".as_ref(),
        "--exact".as_ref(),
    ]);
    assert!(boundary_exact.status.success(), "{boundary_exact:?}");
    let boundary_stdout_raw = String::from_utf8(boundary_exact.stdout).expect("utf8 stdout");
    let boundary_stdout = normalize_temp_root(&boundary_stdout_raw, root);
    assert!(
        boundary_stdout.starts_with(
            "[search-fzf] q=rCAH mode=exact backend=provider pkg=. own=0 finder=fzf fzfArgs=--exact"
        ),
        "{boundary_stdout}"
    );

    let exact_json = run_cli([
        "search".as_ref(),
        "fzf".as_ref(),
        "runCodexAgentHook".as_ref(),
        "--json".as_ref(),
        "--fzf-arg".as_ref(),
        "--exact".as_ref(),
        root.as_os_str(),
    ]);
    assert!(exact_json.status.success(), "{exact_json:?}");
    let value = serde_json::from_slice::<Value>(&exact_json.stdout).expect("fzf exact json");
    assert_eq!(value["finder"]["engine"], "fzf");
    assert_eq!(value["finder"]["surface"], "search-fzf");
    assert_eq!(value["finder"]["options"]["matchMode"], "exact");
    assert_eq!(value["finder"]["options"]["nativeArgs"][0], "--exact");

    let rejected_fzf = run_cli([
        "search".as_ref(),
        "fzf".as_ref(),
        "runCodexAgentHook".as_ref(),
        "--fzf-arg".as_ref(),
        "--preview".as_ref(),
        root.as_os_str(),
    ]);
    assert!(!rejected_fzf.status.success(), "{rejected_fzf:?}");
    let stderr = String::from_utf8(rejected_fzf.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("unsupported fzf option for agent search: --preview"),
        "{stderr}"
    );

    let text = run_cli([
        "search".as_ref(),
        "text".as_ref(),
        "runCodexAgentHook".as_ref(),
        root.as_os_str(),
    ]);
    assert!(!text.status.success(), "{text:?}");
    let stderr = String::from_utf8(text.stderr).expect("utf8 stderr");
    assert!(stderr.contains("unknown search view: text"), "{stderr}");
}

#[test]
fn cli_search_views_render_rfc_line_protocol() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let deps = run_search(root, &["deps"]);
    assert!(deps.starts_with("[search-deps] pkg=. dep=2"), "{deps}");
    assert!(
        deps.contains(
            "|dep serde import=serde pkg=serde version=1 kind=normal opt=true source=manifest manager=cargo feat=derive"
        ),
        "{deps}"
    );
    assert!(
        deps.contains(
            "|dep anyhow import=anyhow pkg=anyhow version=1 kind=normal opt=false source=manifest manager=cargo"
        ),
        "{deps}"
    );

    let dep = run_search(root, &["deps", "serde"]);
    assert!(
        dep.starts_with("[search-deps] q=serde pkg=. dep=1 own=2 api=0"),
        "{dep}"
    );
    assert!(
        dep.contains("|dep serde import=serde pkg=serde version=1"),
        "{dep}"
    );
    assert!(
        dep.contains("|owner src/lib.rs hit_kind=dependency"),
        "{dep}"
    );
    assert!(
        dep.contains("|owner src/domain/mod.rs hit_kind=dependency"),
        "{dep}"
    );

    let dep_usage = run_search(root, &["deps", "serde", "usage"]);
    assert!(
        dep_usage.starts_with("[search-deps] q=serde pkg=. dep=1 own=2 api=0"),
        "{dep_usage}"
    );
    assert!(
        dep_usage.contains("|owner src/lib.rs hit_kind=dependency"),
        "{dep_usage}"
    );
    assert!(
        dep_usage.contains("|owner src/domain/mod.rs hit_kind=dependency"),
        "{dep_usage}"
    );

    let dep_public_api = run_search(root, &["deps", "serde", "public-api"]);
    assert!(
        dep_public_api.starts_with("[search-deps] q=serde pkg=. dep=1 own=2 api=8"),
        "{dep_public_api}"
    );
    assert!(
        dep_public_api.contains("|api src/domain/mod.rs line=4 dep=serde kind=struct name=Thing"),
        "{dep_public_api}"
    );

    let features = run_search(root, &["features"]);
    assert!(
        features.starts_with("[search-features] pkg=. feat=2"),
        "{features}"
    );
    assert!(
        features
            .contains("|feature json enables=dep:serde,serde/derive source=manifest manager=cargo"),
        "{features}"
    );

    let feature = run_search(root, &["features", "json", "cfg", "owners", "tests"]);
    assert!(
        feature.starts_with("[search-features] q=json pkg=. feat=1 dep=1"),
        "{feature}"
    );
    assert!(feature.contains(" cfg=1"), "{feature}");
    assert!(feature.contains(" own=1"), "{feature}");
    assert!(
        feature
            .contains("|feature json enables=dep:serde,serde/derive source=manifest manager=cargo"),
        "{feature}"
    );
    assert!(
        feature.contains("|cfg feature:json declared_in=features expr=cfg(feature=\"json\")"),
        "{feature}"
    );
    assert!(
        feature.contains("|owner src/domain/mod.rs hit_kind=feature"),
        "{feature}"
    );
    assert!(
        feature.contains("|next cfg:json,text:json(scope=src),tests"),
        "{feature}"
    );

    let workspace = run_search(root, &["workspace"]);
    assert!(
        workspace.starts_with("[search-workspace] root=. pkg=1"),
        "{workspace}"
    );
    assert!(
        workspace.contains("|package . root=. manifest=Cargo.toml"),
        "{workspace}"
    );

    let targets = run_search(root, &["targets"]);
    assert!(
        targets.starts_with("[search-targets] pkg=. source=1 test=1"),
        "{targets}"
    );
    assert!(
        targets.contains(
            "|target path=src/lib.rs source=manifest manager=cargo next=owner:src/lib.rs"
        ),
        "{targets}"
    );
    assert!(
        targets.contains(
            "|target path=tests/domain.rs source=manifest manager=cargo next=owner:tests/domain.rs"
        ),
        "{targets}"
    );

    let dependency = run_search(
        root,
        &[
            "dependency",
            "serde",
            "items",
            "public-api",
            "docs",
            "tests",
        ],
    );
    assert!(
        dependency.starts_with("[search-dependency] q=serde pkg=. dep=1 own=2 api=8"),
        "{dependency}"
    );
    assert!(dependency.contains(" item="), "{dependency}");
    assert!(dependency.contains(" docs=8"), "{dependency}");
    assert!(dependency.contains(" tests=1"), "{dependency}");
    assert!(
        dependency.contains("|dep serde import=serde pkg=serde"),
        "{dependency}"
    );
    assert!(
        dependency.contains("|owner src/lib.rs hit_kind=dependency"),
        "{dependency}"
    );
    assert!(
        dependency.contains("|owner src/domain/mod.rs hit_kind=dependency"),
        "{dependency}"
    );
    assert!(dependency.contains("|item load kind=fn"), "{dependency}");
    assert!(
        dependency.contains(
            "|api src/domain/mod.rs line=4 dep=serde kind=struct name=Thing public=true doc=false reason=dependency-owner next=docs:Thing,tests"
        ),
        "{dependency}"
    );
    assert!(
        dependency.contains(
            "|test tests/domain.rs functions=1 owner=src/lib.rs reason=symbol:load next=owner:tests/domain.rs"
        ),
        "{dependency}"
    );
    assert!(
        dependency.contains("|next deps:serde,import:serde,tests"),
        "{dependency}"
    );

    let dependency_set = run_search(root, &["dependency", "serde,anyhow", "items", "tests"]);
    assert!(
        dependency_set.starts_with(
            "[search-dependency] q=serde,anyhow querySet=2 selector=exact-set pkg=. dep=2 own=2 api=0"
        ),
        "{dependency_set}"
    );
    assert!(dependency_set.contains(" item="), "{dependency_set}");
    assert!(dependency_set.contains(" tests=1"), "{dependency_set}");
    assert!(
        dependency_set.contains("|dep serde import=serde pkg=serde"),
        "{dependency_set}"
    );
    assert!(
        dependency_set.contains("|dep anyhow import=anyhow pkg=anyhow"),
        "{dependency_set}"
    );
    assert!(
        dependency_set.contains("|owner src/lib.rs hit_kind=dependency"),
        "{dependency_set}"
    );
    assert!(
        dependency_set.contains("|next deps:serde,import:serde,tests"),
        "{dependency_set}"
    );
    assert!(
        dependency_set.contains("|next deps:anyhow,import:anyhow,tests"),
        "{dependency_set}"
    );

    let dependency_set_json = run_cli([
        "search".as_ref(),
        "dependency".as_ref(),
        "serde,anyhow".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(
        dependency_set_json.status.success(),
        "{dependency_set_json:?}"
    );
    let value =
        serde_json::from_slice::<Value>(&dependency_set_json.stdout).expect("dependency set json");
    assert_eq!(value["query"], "serde,anyhow");
    assert_eq!(value["header"]["fields"]["querySet"], 2);
    assert_eq!(value["header"]["fields"]["selector"], "exact-set");
    assert_eq!(value["querySet"][0]["value"], "serde");
    assert_eq!(value["querySet"][0]["kind"], "dependency");
    assert_eq!(value["querySet"][0]["selector"], "exact");
    assert_eq!(value["querySet"][1]["value"], "anyhow");
    assert_eq!(value["queryComposition"]["mode"], "query-set");
    assert_eq!(value["queryComposition"]["view"], "dependency");
    assert_eq!(value["queryComposition"]["selector"], "exact-set");

    let owner = run_search(root, &["owner", "src/lib.rs", "items"]);
    assert!(
        owner.starts_with("[search-owner] q=src/lib.rs pkg=."),
        "{owner}"
    );
    assert!(owner.contains("|owner src/lib.rs"), "{owner}");
    assert!(owner.contains("|item load kind=fn"), "{owner}");
    assert!(
        owner.contains("|item WireApi kind=trait public=true"),
        "{owner}"
    );
    assert!(!owner.contains("|synthesis"), "{owner}");

    let owner_json = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(owner_json.status.success(), "{owner_json:?}");
    let value = serde_json::from_slice::<Value>(&owner_json.stdout).expect("owner json");
    assert_eq!(
        value["searchSynthesis"]["algorithm"],
        "bounded-reachability-depth1"
    );
    assert_eq!(value["searchSynthesis"]["scope"], "owner");
    assert_eq!(value["searchSynthesis"]["ownerPath"], "src/lib.rs");

    let owner_item_query = run_search(root, &["owner", "src/lib.rs", "items", "--query", "load"]);
    assert!(
        owner_item_query
            .starts_with("[search-owner] q=src/lib.rs pkg=. own=1 item=1 itemQuery=load"),
        "{owner_item_query}"
    );
    assert!(
        owner_item_query
            .contains("|item load kind=fn public=true next=symbol:load read=src/lib.rs:6:6"),
        "{owner_item_query}"
    );
    assert!(
        owner_item_query.contains(
            "|code path=src/lib.rs lineRange=6:6 reason=item-query truncated=false nodes=n0:fn:declaration:0:6:6,n1:call:call:1:6:6 text=\"pub fn load() -> Thing\\ncall domain::make_thing\""
        ),
        "{owner_item_query}"
    );
    assert!(
        !owner_item_query.contains("clone_value"),
        "{owner_item_query}"
    );
    let owner_names_only = run_search(
        root,
        &[
            "owner",
            "src/lib.rs",
            "items",
            "--query",
            "loa",
            "--names-only",
        ],
    );
    assert!(
        owner_names_only.contains(
            "|query itemQuery=loa status=hit match=fallback-contains item=1 reason=parser-item-fallback output=names next=code"
        ),
        "{owner_names_only}"
    );
    assert!(
        owner_names_only.contains("|item load kind=fn"),
        "{owner_names_only}"
    );
    assert!(
        !owner_names_only.contains("|code path=src/lib.rs"),
        "{owner_names_only}"
    );
    let owner_multi_query = run_search(
        root,
        &[
            "owner",
            "src/lib.rs",
            "items",
            "--query",
            "load|clone_value",
        ],
    );
    assert!(
        owner_multi_query.starts_with(
            "[search-owner] q=src/lib.rs pkg=. own=1 item=2 itemQuery=load|clone_value"
        ),
        "{owner_multi_query}"
    );
    assert!(
        owner_multi_query.contains("|item load kind=fn public=true"),
        "{owner_multi_query}"
    );
    assert!(
        owner_multi_query.contains("|item clone_value kind=fn public=true"),
        "{owner_multi_query}"
    );
    assert!(
        owner_multi_query
            .contains("text=\"pub fn clone_value(input: String) -> String\\ncall clone\""),
        "{owner_multi_query}"
    );
    let owner_set = run_search(root, &["owner", "src/lib.rs,src/domain/mod.rs", "items"]);
    assert!(
        owner_set.starts_with(
            "[search-owner] q=src/lib.rs,src/domain/mod.rs querySet=2 selector=exact-set pkg=. own=2"
        ),
        "{owner_set}"
    );
    assert!(owner_set.contains("|owner src/lib.rs"), "{owner_set}");
    assert!(
        owner_set.contains("|owner src/domain/mod.rs"),
        "{owner_set}"
    );
    assert!(owner_set.contains("|item load kind=fn"), "{owner_set}");
    assert!(
        owner_set.contains("|item make_thing kind=fn"),
        "{owner_set}"
    );

    let symbol = run_search(root, &["symbol", "load"]);
    assert!(
        symbol.starts_with("[search-symbol] q=load pkg=."),
        "{symbol}"
    );
    assert!(symbol.contains("|def src/lib.rs"), "{symbol}");
    assert!(symbol.contains("kind=fn name=load"), "{symbol}");

    let callsite = run_search(root, &["callsite", "make_thing"]);
    assert!(
        callsite.starts_with("[search-callsite] q=make_thing pkg=."),
        "{callsite}"
    );
    assert!(callsite.contains("|call src/lib.rs"), "{callsite}");

    let import = run_search(root, &["import", "serde"]);
    assert!(
        import.starts_with("[search-import] q=serde pkg=. own=2"),
        "{import}"
    );
    assert!(
        import.contains("|owner src/lib.rs hit_kind=import"),
        "{import}"
    );
    assert!(
        import.contains("|owner src/domain/mod.rs hit_kind=import"),
        "{import}"
    );

    let fzf = run_search(root, &["fzf", "Thing", "--scope", "src"]);
    assert!(
        fzf.starts_with("[search-fzf] q=Thing mode=fuzzy backend=provider pkg=. own=2"),
        "{fzf}"
    );
    assert!(fzf.contains("|owner src/lib.rs hit_kind=fzf"), "{fzf}");
    assert!(
        fzf.contains("|owner src/domain/mod.rs hit_kind=fzf"),
        "{fzf}"
    );

    let fzf_set = run_cli([
        "search".as_ref(),
        "fzf".as_ref(),
        "--query-set".as_ref(),
        "Thing".as_ref(),
        "--query-set".as_ref(),
        "make_thing".as_ref(),
        "--scope".as_ref(),
        "src".as_ref(),
        root.as_os_str(),
    ]);
    assert!(fzf_set.status.success(), "{fzf_set:?}");
    let fzf_set_stdout = String::from_utf8(fzf_set.stdout).expect("utf8 stdout");
    assert!(
        fzf_set_stdout.starts_with(
            "[search-fzf] q=Thing,make_thing querySet=2 selector=fuzzy-set mode=fuzzy backend=provider pkg=. own=2"
        ),
        "{fzf_set_stdout}"
    );
    assert!(
        fzf_set_stdout.contains("|owner src/lib.rs hit_kind=fzf querySet=2"),
        "{fzf_set_stdout}"
    );
    assert!(
        fzf_set_stdout.contains("window_set=")
            && fzf_set_stdout.contains("owner:src/lib.rs")
            && fzf_set_stdout.contains("owner:src/domain/mod.rs"),
        "{fzf_set_stdout}"
    );

    let fzf_set_json = run_cli([
        "search".as_ref(),
        "fzf".as_ref(),
        "--query-set".as_ref(),
        "Thing".as_ref(),
        "--query-set".as_ref(),
        "make_thing".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(fzf_set_json.status.success(), "{fzf_set_json:?}");
    let value = serde_json::from_slice::<Value>(&fzf_set_json.stdout).expect("fzf set json");
    assert_eq!(value["query"], "Thing,make_thing");
    assert_eq!(value["querySet"][0]["value"], "Thing");
    assert_eq!(value["querySet"][0]["kind"], "text");
    assert_eq!(value["querySet"][1]["value"], "make_thing");
    assert_eq!(value["queryComposition"]["mode"], "query-set");

    let fzf_set_frontier = run_cli([
        "search".as_ref(),
        "fzf".as_ref(),
        "--query-set".as_ref(),
        "load".as_ref(),
        "--query-set".as_ref(),
        "Thing".as_ref(),
        "--view".as_ref(),
        "seeds".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(fzf_set_frontier.status.success(), "{fzf_set_frontier:?}");
    let value = serde_json::from_slice::<Value>(&fzf_set_frontier.stdout).expect("frontier json");
    assert_eq!(
        value["searchSynthesis"]["algorithm"],
        "change-frontier-query-set"
    );
    assert_eq!(value["searchSynthesis"]["scope"], "query-set");
    assert!(
        value["searchSynthesis"]["editFrontier"]
            .as_array()
            .expect("edit frontier")
            .iter()
            .any(|path| path.as_str() == Some("src/lib.rs")),
        "{value}"
    );
    assert!(
        value["searchSynthesis"]["testFrontier"]
            .as_array()
            .expect("test frontier")
            .iter()
            .any(|path| path.as_str() == Some("tests/domain.rs")),
        "{value}"
    );
    assert!(
        value["searchSynthesis"]["windowSet"]
            .as_array()
            .expect("window set")
            .iter()
            .any(|window| window["kind"] == "owner" && window["target"] == "src/lib.rs"),
        "{value}"
    );
    assert!(
        value["searchSynthesis"]["windowSet"]
            .as_array()
            .expect("window set")
            .iter()
            .any(|window| window["kind"] == "tests" && window["target"] == "tests/domain.rs"),
        "{value}"
    );
    assert!(
        value["searchSynthesis"]["seeds"]
            .as_array()
            .expect("frontier seeds")
            .iter()
            .any(|seed| seed["kind"] == "owner" && seed["target"] == "src/lib.rs"),
        "{value}"
    );

    let owner_with_tests = run_search(root, &["owner", "src/lib.rs", "items", "tests"]);
    assert!(
        owner_with_tests.contains("|item load kind=fn public=true"),
        "{owner_with_tests}"
    );
    assert!(
        owner_with_tests
            .contains("|test tests/domain.rs functions=1 owner=src/lib.rs reason=symbol:load"),
        "{owner_with_tests}"
    );

    let cfg = run_search(root, &["cfg", "json"]);
    assert!(
        cfg.starts_with("[search-cfg] q=json pkg=. cfg=1 dep=0 own=1"),
        "{cfg}"
    );

    let patterns = run_search(root, &["patterns"]);
    assert!(patterns.starts_with("[search-patterns] n=8"), "{patterns}");
    assert!(patterns.contains("|pat clone-in-loop"), "{patterns}");
    assert!(
        patterns.contains("|pat public-error-boundary lang=rust scope=src"),
        "{patterns}"
    );
    assert!(
        patterns.contains("|pat public-external-type lang=rust scope=src option=dependency"),
        "{patterns}"
    );

    let pattern = run_search(root, &["pattern", "clone-in-loop"]);
    assert!(
        pattern.starts_with("[search-pattern] pattern=clone-in-loop q=.clone("),
        "{pattern}"
    );
    assert!(
        pattern.contains("|owner src/lib.rs hit_kind=fzf"),
        "{pattern}"
    );

    let anyhow_pattern = run_search(root, &["pattern", "public-anyhow-result"]);
    assert!(
        anyhow_pattern.starts_with(
            "[search-pattern] pattern=public-anyhow-result pkg=. hit=2 source=native-parser"
        ),
        "{anyhow_pattern}"
    );
    assert!(
        anyhow_pattern.contains(
            "|api src/lib.rs:11 kind=fn name=fallible next=owner:src/lib.rs source=native-parser signature=fn(input:String)->anyhow::Result<Thing> params=input:String async=true unsafe=true receiver=- return=anyhow::Result<Thing> error=anyhow::Result"
        ),
        "{anyhow_pattern}"
    );
    assert!(
        anyhow_pattern.contains(
            "|api src/lib.rs:18 kind=method name=wire next=owner:src/lib.rs source=native-parser signature=fn()->anyhow::Result<Thing> params=- async=false unsafe=false receiver=&self return=anyhow::Result<Thing> error=anyhow::Result impl=PublicWire trait=WireApi"
        ),
        "{anyhow_pattern}"
    );
    assert!(
        !anyhow_pattern.contains("hit_kind=text"),
        "{anyhow_pattern}"
    );

    let error_boundary_pattern = run_search(root, &["pattern", "public-error-boundary"]);
    assert!(
        error_boundary_pattern.starts_with(
            "[search-pattern] pattern=public-error-boundary pkg=. hit=2 source=native-parser"
        ),
        "{error_boundary_pattern}"
    );
    assert!(
        error_boundary_pattern.contains("name=fallible"),
        "{error_boundary_pattern}"
    );
    assert!(
        error_boundary_pattern.contains("name=wire"),
        "{error_boundary_pattern}"
    );
    assert!(
        error_boundary_pattern.contains("source=native-parser"),
        "{error_boundary_pattern}"
    );

    let external_pattern = run_search(
        root,
        &["pattern", "public-external-type", "--dependency", "serde"],
    );
    assert!(
        external_pattern.starts_with(
            "[search-pattern] pattern=public-external-type q=serde pkg=. dep=1 own=2 api=8"
        ),
        "{external_pattern}"
    );
    assert!(
        external_pattern.contains("|api src/domain/mod.rs line=4 dep=serde kind=struct name=Thing"),
        "{external_pattern}"
    );

    let api_shape = run_search(
        root,
        &["pattern", "public-api-shape", "--owner", "src/lib.rs"],
    );
    assert!(
        api_shape.starts_with(
            "[search-pattern] pattern=public-api-shape q=src/lib.rs pkg=. own=1 item=6"
        ),
        "{api_shape}"
    );
    assert!(api_shape.contains("|item load kind=fn"), "{api_shape}");

    let docs = run_search(root, &["docs", "Thing"]);
    assert!(
        docs.starts_with("[search-docs] q=Thing pkg=. docs=1 source=native-parser"),
        "{docs}"
    );
    assert!(docs.contains("|api src/domain/mod.rs"), "{docs}");
    assert!(docs.contains("kind=struct name=Thing"), "{docs}");

    let fallible_docs = run_search(root, &["docs", "fallible"]);
    assert!(
        fallible_docs.starts_with("[search-docs] q=fallible pkg=. docs=1 source=native-parser"),
        "{fallible_docs}"
    );
    assert!(
        fallible_docs.contains(
            "|api src/lib.rs:11 kind=fn name=fallible next=owner:src/lib.rs source=native-parser docs=local-parser signature=fn(input:String)->anyhow::Result<Thing> params=input:String async=true unsafe=true receiver=- return=anyhow::Result<Thing> error=anyhow::Result"
        ),
        "{fallible_docs}"
    );

    let api = run_search(root, &["api", "fallible"]);
    assert!(
        api.starts_with("[search-api] q=fallible pkg=. api=1 source=native-parser"),
        "{api}"
    );
    assert!(
        api.contains(
            "|api src/lib.rs:11 kind=fn name=fallible next=owner:src/lib.rs source=native-parser signature=fn(input:String)->anyhow::Result<Thing> params=input:String async=true unsafe=true receiver=- return=anyhow::Result<Thing> error=anyhow::Result"
        ),
        "{api}"
    );

    let method_api = run_search(root, &["api", "as_thing"]);
    assert!(
        method_api.starts_with("[search-api] q=as_thing pkg=. api=1 source=native-parser"),
        "{method_api}"
    );
    assert!(
        method_api.contains(
            "|api src/lib.rs:15 kind=method name=as_thing next=owner:src/lib.rs source=native-parser signature=fn()->Thing params=- async=false unsafe=false receiver=&mut-self return=Thing error=- impl=PublicWire trait=-"
        ),
        "{method_api}"
    );

    let trait_method_api = run_search(root, &["api", "wire"]);
    assert!(
        trait_method_api.starts_with("[search-api] q=wire pkg=. api=1 source=native-parser"),
        "{trait_method_api}"
    );
    assert!(
        trait_method_api.contains(
            "|api src/lib.rs:18 kind=method name=wire next=owner:src/lib.rs source=native-parser signature=fn()->anyhow::Result<Thing> params=- async=false unsafe=false receiver=&self return=anyhow::Result<Thing> error=anyhow::Result impl=PublicWire trait=WireApi"
        ),
        "{trait_method_api}"
    );

    let external_types = run_search(root, &["public-external-types"]);
    assert!(
        external_types.starts_with("[search-public-external-types] pkg=. dep=2 hit=3"),
        "{external_types}"
    );
    assert!(
        external_types.contains(
            "|external-type src/lib.rs:11 dep=anyhow surface=return item=fallible type=anyhow::Result<Thing> source=native-parser next=dependency:anyhow,docs:anyhow::Result<Thing>"
        ),
        "{external_types}"
    );
    assert!(
        external_types.contains(
            "|external-type src/lib.rs:13 dep=serde surface=field:serializer item=PublicWire type=serde::Serialize source=native-parser next=dependency:serde,docs:serde::Serialize"
        ),
        "{external_types}"
    );

    let serde_external_types =
        run_search(root, &["public-external-types", "--dependency", "serde"]);
    assert!(
        serde_external_types.starts_with("[search-public-external-types] pkg=. dep=1 hit=1"),
        "{serde_external_types}"
    );
    assert!(
        serde_external_types.contains("dep=serde surface=field:serializer"),
        "{serde_external_types}"
    );

    let docs_use = run_search(root, &["docs-use", "load"]);
    assert!(
        docs_use.contains("[search-docs] q=load pkg=. docs=1 source=native-parser"),
        "{docs_use}"
    );
    assert!(
        docs_use.contains("[search-callsite] q=load pkg=."),
        "{docs_use}"
    );

    let dependency_docs = run_search(root, &["docs", "serde::Serialize"]);
    assert!(
        dependency_docs.starts_with(
            "[search-docs] q=serde::Serialize pkg=. docs=0 source=registry-source crate=serde"
        ),
        "{dependency_docs}"
    );
    assert!(
        dependency_docs.contains("|note docsSource=registry-source missing=true"),
        "{dependency_docs}"
    );

    let current_version_api = run_search(root, &["api", "serde@1::Serialize"]);
    assert!(
        current_version_api.starts_with(
            "[search-api] q=serde@1::Serialize pkg=. api=0 source=registry-source crate=serde requestedVersion=1 versionScope=current currentWorkspaceVersion=1"
        ),
        "{current_version_api}"
    );

    let external_docs = run_search(root, &["docs", "serde@2::Serialize"]);
    assert!(
        external_docs.starts_with(
            "[search-docs] q=serde@2::Serialize pkg=. docs=0 source=registry-source crate=serde requestedVersion=2 versionScope=external currentWorkspaceVersion=1"
        ),
        "{external_docs}"
    );
    assert!(
        external_docs.contains("|note docsSource=registry-source missing=true"),
        "{external_docs}"
    );
    assert!(!external_docs.contains("|api "), "{external_docs}");

    let external_api_json = run_cli([
        "search".as_ref(),
        "api".as_ref(),
        "serde@2::Serialize".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(external_api_json.status.success(), "{external_api_json:?}");
    let value =
        serde_json::from_slice::<Value>(&external_api_json.stdout).expect("external api json");
    let header_fields = value["header"]["fields"]
        .as_object()
        .expect("header fields");
    assert_eq!(header_fields["source"], "registry-source");
    assert_eq!(header_fields["crate"], "serde");
    assert_eq!(header_fields["requestedVersion"], "2");
    assert_eq!(header_fields["versionScope"], "external");
    assert_eq!(header_fields["currentWorkspaceVersion"], "1");
    let note_fields = value["notes"][0]["fields"]
        .as_object()
        .expect("note fields");
    assert_eq!(note_fields["apiSource"], "registry-source");

    let tests = run_search(root, &["tests", "domain"]);
    assert!(
        tests.starts_with("[search-tests] q=domain pkg=. tests=1"),
        "{tests}"
    );
    assert!(
        tests.contains("|test tests/domain.rs functions=1 next=owner:tests/domain.rs"),
        "{tests}"
    );

    let owner_tests = run_search(root, &["tests", "src/lib.rs"]);
    assert!(
        owner_tests.starts_with("[search-tests] q=src/lib.rs pkg=. tests=1 own=1"),
        "{owner_tests}"
    );
    assert!(
        owner_tests.contains("|node O:src/lib.rs kind=owner path=src/lib.rs"),
        "{owner_tests}"
    );
    assert!(
        owner_tests.contains(
            "|test tests/domain.rs functions=1 owner=src/lib.rs reason=symbol:load next=owner:tests/domain.rs"
        ),
        "{owner_tests}"
    );

    let tests_set = run_cli([
        "search".as_ref(),
        "tests".as_ref(),
        "--query-set".as_ref(),
        "src/lib.rs".as_ref(),
        "--query-set".as_ref(),
        "src/domain/mod.rs".as_ref(),
        root.as_os_str(),
    ]);
    assert!(tests_set.status.success(), "{tests_set:?}");
    let tests_set_stdout = String::from_utf8(tests_set.stdout).expect("utf8 stdout");
    assert!(
        tests_set_stdout.starts_with("[search-tests] q=src/lib.rs,src/domain/mod.rs querySet=2"),
        "{tests_set_stdout}"
    );
    assert!(
        tests_set_stdout.contains("|node O:src/lib.rs kind=owner path=src/lib.rs"),
        "{tests_set_stdout}"
    );
    assert!(
        tests_set_stdout.contains("|node O:src/domain/mod.rs kind=owner path=src/domain/mod.rs"),
        "{tests_set_stdout}"
    );
    assert!(
        owner_tests.contains("|edge O:src/lib.rs -test-> T:tests/domain.rs"),
        "{owner_tests}"
    );

    let ingest = run_search_with_stdin(
        root,
        &["ingest", "items"],
        "src/lib.rs:4:pub fn load() -> Thing\n",
    );
    assert!(
        ingest.starts_with("[search-ingest] src=rg-n in=1 own=1"),
        "{ingest}"
    );
    assert!(
        ingest.contains("|owner src/lib.rs role=source hit_kind=text locations=4:1 next=owner"),
        "{ingest}"
    );

    let ingest_seeds = run_search_with_stdin(
        root,
        &[
            "ingest", "items", "tests", "--view", "seeds", "--seeds", "8",
        ],
        "src/lib.rs:6:pub fn load() -> Thing\n",
    );
    assert!(
        ingest_seeds.starts_with("[search-ingest] src=rg-n in=1 own=1"),
        "{ingest_seeds}"
    );
    assert!(
        ingest_seeds.contains("|seed owner:src/lib.rs"),
        "{ingest_seeds}"
    );
    assert!(
        ingest_seeds.contains("|seed tests:src/lib.rs"),
        "{ingest_seeds}"
    );
    assert!(ingest_seeds.contains("|seed symbol:load"), "{ingest_seeds}");
    assert!(
        !ingest_seeds.contains("|owner src/lib.rs"),
        "{ingest_seeds}"
    );
}
