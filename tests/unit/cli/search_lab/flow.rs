use tempfile::TempDir;

use super::{FORBIDDEN_FLOW_PATTERNS, assert_lab_packet};
use crate::cli::support::{
    run_search, run_search_with_stdin, write_complex_dependency_fixture, write_search_fixture,
};

#[test]
fn search_lab_multi_pipe_dependency_flow_compresses_to_final_packet() {
    if crate::cli::support::skip_if_protocol_graph_renderer_unavailable() {
        return;
    }

    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let full = run_search(
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
    assert_lab_packet(
        "dependency_multi_pipe_final_packet",
        &full,
        27,
        &[
            "[search-dependency] q=serde pkg=. dep=1 own=2 api=8 item=",
            " docs=8 tests=1",
            "|dep serde import=serde pkg=serde version=1 kind=normal opt=true source=manifest manager=cargo feat=derive",
            "|owner src/lib.rs hit_kind=dependency locations=",
            "|owner src/domain/mod.rs hit_kind=dependency locations=",
            "|item load kind=fn responsibilities=early-return public=true next=syntax:load",
            "|api src/domain/mod.rs line=4 dep=serde kind=struct name=Thing",
            "|test tests/domain.rs functions=1 owner=src/lib.rs",
            "|next deps:serde,import:serde,docs-use:serde,crate-source:serde,tests",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let seeds = run_search(
        root,
        &[
            "dependency",
            "serde",
            "items",
            "public-api",
            "docs",
            "tests",
            "--view",
            "seeds",
        ],
    );
    assert!(
        seeds.lines().count() < full.lines().count(),
        "seeds view should be smaller than final packet:\n{seeds}\n--- full ---\n{full}"
    );
    assert_lab_packet(
        "dependency_multi_pipe_seed_packet",
        &seeds,
        10,
        &[
            "[search-dependency] q=serde alg=seed-frontier",
            "D=dependency:pkg(serde)!dependency",
            "rank=D,",
            "frontier=D.dependency",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );
    assert!(
        !seeds.contains("|owner src/lib.rs"),
        "seeds view should not re-emit owner detail:\n{seeds}"
    );
    assert!(
        !seeds.contains("|api src/domain/mod.rs"),
        "seeds view should not re-emit API detail:\n{seeds}"
    );

    let tight_seeds = run_search(
        root,
        &[
            "dependency",
            "serde",
            "items",
            "public-api",
            "docs",
            "tests",
            "--view",
            "seeds",
            "--seeds",
            "3",
        ],
    );
    assert_lab_packet(
        "dependency_multi_pipe_tight_seed_packet",
        &tight_seeds,
        7,
        &[
            "[search-dependency] q=serde alg=seed-frontier",
            "aliases: graph:{G=search,D=dependency",
            "D=dependency:pkg(serde)!dependency",
            "G>{D:uses",
            "rank=D,",
            "frontier=D.dependency",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );
}

#[test]
fn search_lab_feature_cfg_flow_routes_dependency_owner_and_tests() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_complex_dependency_fixture(root);

    let feature = run_search(root, &["features", "runtime", "cfg", "owners", "tests"]);
    assert_lab_packet(
        "feature_cfg_owner_tests_flow",
        &feature,
        18,
        &[
            "[search-features] q=runtime pkg=. feat=1 dep=2 cfg=1 own=2 tests=1",
            "|feature runtime enables=dep:tokio,tokio/rt-multi-thread,tokio/sync,dep:bytes source=manifest manager=cargo",
            "|dep tokio import=tokio pkg=tokio version=1 kind=normal opt=true source=manifest manager=cargo",
            "|dep bytes import=bytes pkg=bytes version=1 kind=normal opt=true source=manifest manager=cargo",
            "|cfg feature:runtime declared_in=features expr=cfg(feature=\"runtime\")",
            "|owner src/lib.rs hit_kind=feature locations=",
            "|owner src/http/client.rs hit_kind=feature locations=",
            "|test tests/flow.rs functions=1 owner=src/http/client.rs",
            "|next cfg:runtime,text:runtime(scope=src),tests",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );
}

#[test]
fn search_lab_ingest_folds_raw_hits_into_owner_items_and_tests() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let ingest = run_search_with_stdin(
        root,
        &["ingest", "items", "tests"],
        "src/lib.rs:6:pub fn load() -> Thing { domain::make_thing() }\n\
         src/domain/mod.rs:4:pub struct Thing { pub id: String }\n",
    );
    assert_lab_packet(
        "ingest_rg_n_owner_items_tests",
        &ingest,
        12,
        &[
            "[search-ingest] src=rg-n in=2 own=2",
            "|owner src/lib.rs role=source hit_kind=text locations=6:1 next=owner",
            "|item load kind=fn responsibilities=early-return public=true next=syntax:load",
            "|owner src/domain/mod.rs role=source hit_kind=text locations=4:1 next=owner",
            "|item Thing kind=struct responsibilities=data-shape public=true next=syntax:Thing",
            "|test tests/domain.rs functions=1 owner=src/lib.rs",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );
}

#[test]
fn search_lab_deps_flow_separates_current_workspace_and_external_versions() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_complex_dependency_fixture(root);

    let current = run_search(root, &["deps", "tokio@1::Sender"]);
    assert_lab_packet(
        "deps_current_workspace_api",
        &current,
        8,
        &[
            "[search-deps] q=tokio@1::Sender pkg=. dep=1 own=1 api=0 requestedVersion=1 currentWorkspaceVersion=1 versionScope=current apiQuery=Sender",
            "|dep tokio import=tokio pkg=tokio version=1 kind=normal opt=true source=manifest manager=cargo",
            "|dependency-guidance dep=tokio usageLevel=basic_usage engineeringBoundary=missing",
            "|owner src/http/client.rs hit_kind=dependency-api apiQuery=Sender",
            "|next dependency:tokio,docs-use:tokio::Sender,crate-source:tokio,import:tokio,tests:Sender",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let external = run_search(root, &["deps", "ignore@0.3::WalkBuilder"]);
    assert_lab_packet(
        "deps_external_version_api",
        &external,
        8,
        &[
            "[search-deps] q=ignore@0.3::WalkBuilder pkg=. dep=1 own=0 api=0 requestedVersion=0.3 currentWorkspaceVersion=0.4 versionScope=external apiQuery=WalkBuilder",
            "|note kind=version-scope message=requested-version-is-outside-current-workspace-version",
            "|next dependency:ignore,docs-use:ignore::WalkBuilder,crate-source:ignore,import:ignore,tests:WalkBuilder",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );
    assert!(
        !external.contains("|owner src/io/walk.rs"),
        "external-version search must not reuse current workspace owners:\n{external}"
    );
}
