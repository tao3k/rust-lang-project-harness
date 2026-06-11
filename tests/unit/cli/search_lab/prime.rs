use std::path::Path;

use tempfile::TempDir;

use super::fixtures::{
    write_file_search_ignore_fixture, write_large_rustc_workspace_fixture,
    write_orgize_tracing_fixture, write_tokio_io_uring_fixture,
};
use super::{FORBIDDEN_PRIME_PATTERNS, assert_lab_packet};
use crate::cli::support::{run_search, write_complex_dependency_fixture};

struct PrimeScenario<'a> {
    name: &'a str,
    write_fixture: fn(&Path),
    args: &'a [&'a str],
    max_lines: usize,
    required: &'a [&'a str],
    forbidden: &'a [&'a str],
}

#[test]
fn prime_search_lab_exposes_complex_feature_cfg_dependency_axes() {
    assert_prime_scenario(PrimeScenario {
        name: "tokio_feature_io_uring",
        write_fixture: write_tokio_io_uring_fixture,
        args: &["prime"],
        max_lines: 36,
        required: &[
            "[search-prime] mode=package package=.",
            "|decision purpose=decision-primer",
            "|feature io-uring dep=dep:io-uring,libc,mio/os-poll,mio/os-ext,dep:slab next=features:io-uring",
            "|cfg tokio_unstable",
            "target=cfg(all(tokio_unstable,target_os=\"linux\"))",
            "|dep io-uring import=io_uring pkg=io-uring",
            "|test-surface tests=tests next=tests",
            "|api-candidate IoUringDriver reason=public-item owner=src/io_uring.rs",
        ],
        forbidden: FORBIDDEN_PRIME_PATTERNS,
    });
}

#[test]
fn prime_search_lab_routes_large_workspaces_before_package_prime() {
    assert_prime_scenario(PrimeScenario {
        name: "rustc_workspace_index",
        write_fixture: write_large_rustc_workspace_fixture,
        args: &["prime"],
        max_lines: 10,
        required: &[
            "[search-prime] mode=workspace-index workspace=large packages=14",
            "|package compiler/rustc_middle next=package:compiler/rustc_middle",
            "|package compiler/rustc_type_ir next=package:compiler/rustc_type_ir",
            "|note truncated_packages=6",
        ],
        forbidden: FORBIDDEN_PRIME_PATTERNS,
    });

    assert_prime_scenario(PrimeScenario {
        name: "rustc_middle_smallvec_package",
        write_fixture: write_large_rustc_workspace_fixture,
        args: &["prime", "--package", "compiler/rustc_middle"],
        max_lines: 28,
        required: &[
            "[search-prime] mode=package package=compiler/rustc_middle",
            "|decision purpose=decision-primer",
            "|package compiler/rustc_middle t=lib dep=rustc_type_ir,smallvec,tracing",
            "|dep smallvec import=smallvec pkg=smallvec version=1 kind=normal opt=false source=manifest manager=cargo feat=may_dangle,union",
            "|api-candidate TyCtxt reason=public-item owner=src/ty.rs",
            "|edge O:src/lib.rs -mod-> O:src/ty.rs",
        ],
        forbidden: FORBIDDEN_PRIME_PATTERNS,
    });
}

#[test]
fn prime_search_lab_marks_source_large_dependency_owners() {
    assert_prime_scenario(PrimeScenario {
        name: "codex_file_search_ignore_source_large",
        write_fixture: write_file_search_ignore_fixture,
        args: &["prime"],
        max_lines: 24,
        required: &[
            "[search-prime] mode=package package=.",
            "|decision purpose=decision-primer",
            "|dep ignore import=ignore pkg=ignore version=0.4 kind=normal opt=false source=manifest manager=cargo",
            "|api-candidate build_walker reason=public-item owner=src/lib.rs",
            "|owner src/lib.rs role=root,facade owner=src imports=external:1 source_large=true next=items",
        ],
        forbidden: FORBIDDEN_PRIME_PATTERNS,
    });
}

#[test]
fn prime_search_lab_bounds_tokio_ignore_bytes_dependency_mesh() {
    assert_prime_scenario(PrimeScenario {
        name: "tokio_ignore_bytes_dependency_mesh",
        write_fixture: write_complex_dependency_fixture,
        args: &["prime"],
        max_lines: 39,
        required: &[
            "[search-prime] mode=package package=.",
            "|decision purpose=decision-primer",
            "|package . t=lib,test dep=bytes,ignore,tokio",
            "|feature runtime dep=dep:tokio,tokio/rt-multi-thread,tokio/sync,dep:bytes",
            "|feature walk dep=dep:ignore",
            "|dep bytes import=bytes pkg=bytes version=1 kind=normal opt=true source=manifest manager=cargo",
            "|dep ignore import=ignore pkg=ignore version=0.4 kind=normal opt=true source=manifest manager=cargo",
            "|dep tokio import=tokio pkg=tokio version=1 kind=normal opt=true source=manifest manager=cargo",
            "|api-candidate RuntimeClient reason=public-item owner=src/http/client.rs",
            "|api-candidate WalkPlan reason=public-item owner=src/io/walk.rs",
            "|edge O:src/http/mod.rs -mod-> O:src/http/client.rs",
            "|edge O:src/io/mod.rs -mod-> O:src/io/walk.rs",
        ],
        forbidden: FORBIDDEN_PRIME_PATTERNS,
    });
}

#[test]
fn prime_search_lab_exposes_feature_bound_examples_and_benches() {
    assert_prime_scenario(PrimeScenario {
        name: "orgize_tracing_feature_targets",
        write_fixture: write_orgize_tracing_fixture,
        args: &["prime"],
        max_lines: 26,
        required: &[
            "[search-prime] mode=package package=.",
            "|decision purpose=decision-primer",
            "|package . t=lib,example,bench dep=tracing",
            "|feature tracing dep=dep:tracing next=features:tracing",
            "|target example:parse path=examples/parse.rs required_features=tracing source=manifest manager=cargo next=owner:examples/parse.rs",
            "|target bench:parse path=benches/parse.rs harness=false required_features=- source=manifest manager=cargo next=owner:benches/parse.rs",
            "|api-candidate parse_document reason=public-item owner=src/syntax/mod.rs",
        ],
        forbidden: FORBIDDEN_PRIME_PATTERNS,
    });
}

fn assert_prime_scenario(scenario: PrimeScenario<'_>) {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    (scenario.write_fixture)(root);

    let rendered = run_search(root, scenario.args);
    assert_lab_packet(
        scenario.name,
        &rendered,
        scenario.max_lines,
        scenario.required,
        scenario.forbidden,
    );
}
