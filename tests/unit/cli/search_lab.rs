use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::support::{run_search, write_complex_dependency_fixture};

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
        max_lines: 38,
        required: &[
            "[search-prime] mode=package package=.",
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
            "|package . t=lib,example,bench dep=tracing",
            "|feature tracing dep=dep:tracing next=features:tracing",
            "|target example:parse path=examples/parse.rs required_features=tracing source=manifest manager=cargo next=owner:examples/parse.rs",
            "|target bench:parse path=benches/parse.rs harness=false required_features=- source=manifest manager=cargo next=owner:benches/parse.rs",
            "|api-candidate parse_document reason=public-item owner=src/syntax/mod.rs",
        ],
        forbidden: FORBIDDEN_PRIME_PATTERNS,
    });
}

const FORBIDDEN_PRIME_PATTERNS: &[&str] =
    &["--compact", "intent=", "subagent-plan", "|hit ", "Modules:"];

fn assert_prime_scenario(scenario: PrimeScenario<'_>) {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    (scenario.write_fixture)(root);

    let rendered = run_search(root, scenario.args);
    let line_count = rendered.lines().count();
    assert!(
        line_count <= scenario.max_lines,
        "{} exceeded max_lines={} with {} lines:\n{}",
        scenario.name,
        scenario.max_lines,
        line_count,
        rendered
    );
    for required in scenario.required {
        assert!(
            rendered.contains(required),
            "{} missing required fragment {required:?} in:\n{}",
            scenario.name,
            rendered
        );
    }
    for forbidden in scenario.forbidden {
        assert!(
            !rendered.contains(forbidden),
            "{} contained forbidden fragment {forbidden:?} in:\n{}",
            scenario.name,
            rendered
        );
    }
}

fn write_tokio_io_uring_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"tokio-io-uring-lab\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [lib]\n\
         path = \"src/lib.rs\"\n\n\
         [[test]]\n\
         name = \"io_uring\"\n\
         path = \"tests/io_uring.rs\"\n\n\
         [features]\n\
         default = [\"io-uring\"]\n\
         io-uring = [\"dep:io-uring\", \"libc\", \"mio/os-poll\", \"mio/os-ext\", \"dep:slab\"]\n\n\
         [dependencies]\n\
         io-uring = { version = \"0.7\", optional = true }\n\
         libc = { version = \"0.2\", optional = true }\n\
         mio = { version = \"1\", optional = true, features = [\"os-ext\", \"os-poll\"] }\n\
         slab = { version = \"0.4\", optional = true }\n\n\
         [target.'cfg(all(tokio_unstable, target_os = \"linux\"))'.dependencies]\n\
         io-uring = \"0.7\"\n\
         libc = \"0.2\"\n\
         mio = { version = \"1\", features = [\"os-ext\", \"os-poll\"] }\n\
         slab = \"0.4\"\n\n\
         [lints.rust]\n\
         unexpected_cfgs = { level = \"warn\", check-cfg = ['cfg(tokio_unstable)'] }\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::create_dir_all(root.join("tests")).expect("create tests");
    fs::write(
        root.join("src/lib.rs"),
        "#![cfg_attr(tokio_unstable, allow(dead_code))]\n\
         #[cfg(tokio_unstable)]\n\
         pub mod io_uring;\n\n\
         #[cfg(feature = \"io-uring\")]\n\
         pub fn io_uring_enabled() -> bool { true }\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/io_uring.rs"),
        "use io_uring::opcode;\n\
         pub struct IoUringDriver;\n\
         pub fn register(driver: IoUringDriver) { let _ = opcode::Nop::new(); let _ = driver; }\n",
    )
    .expect("write io_uring");
    fs::write(
        root.join("tests/io_uring.rs"),
        "use tokio_io_uring_lab::io_uring_enabled;\n\
         #[test]\n\
         fn feature_gate_is_visible() { assert!(io_uring_enabled()); }\n",
    )
    .expect("write test");
}

fn write_large_rustc_workspace_fixture(root: &Path) {
    let members = [
        "compiler/rustc_middle",
        "compiler/rustc_type_ir",
        "crates/member00",
        "crates/member01",
        "crates/member02",
        "crates/member03",
        "crates/member04",
        "crates/member05",
        "crates/member06",
        "crates/member07",
        "crates/member08",
        "crates/member09",
        "crates/member10",
        "crates/member11",
    ];
    let mut manifest = String::from("[workspace]\nmembers = [\n");
    for member in members {
        let _ = writeln!(manifest, "  \"{member}\",");
    }
    manifest.push_str("]\n");
    fs::write(root.join("Cargo.toml"), manifest).expect("write workspace manifest");

    write_member_package(
        root,
        "compiler/rustc_type_ir",
        "rustc_type_ir",
        "",
        "pub struct TyKind;\n",
    );
    write_member_package(
        root,
        "compiler/rustc_middle",
        "rustc_middle",
        "[dependencies]\n\
         rustc_type_ir = { path = \"../rustc_type_ir\" }\n\
         smallvec = { version = \"1\", features = [\"union\", \"may_dangle\"] }\n\
         tracing = \"0.1\"\n",
        "mod ty;\n\
         pub use ty::TyCtxt;\n",
    );
    fs::write(
        root.join("compiler/rustc_middle/src/ty.rs"),
        "use smallvec::SmallVec;\n\
         pub struct TyCtxt { pub stack: SmallVec<[usize; 4]> }\n",
    )
    .expect("write rustc_middle ty");
    for index in 0..12 {
        let member = format!("crates/member{index:02}");
        let package = format!("member{index:02}");
        write_member_package(root, &member, &package, "", "pub fn marker() {}\n");
    }
}

fn write_file_search_ignore_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"file-search-lab\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [dependencies]\n\
         ignore = \"0.4\"\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src")).expect("create src");
    let mut source = String::from(
        "use ignore::WalkBuilder;\n\n\
         pub fn build_walker(root: &str) -> WalkBuilder { WalkBuilder::new(root) }\n",
    );
    for index in 0..330 {
        let _ = writeln!(
            source,
            "pub fn generated_route_{index}() -> usize {{ {index} }}"
        );
    }
    fs::write(root.join("src/lib.rs"), source).expect("write large lib");
}

fn write_orgize_tracing_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"orgize-tracing-lab\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [features]\n\
         tracing = [\"dep:tracing\"]\n\n\
         [dependencies]\n\
         tracing = { version = \"0.1\", optional = true }\n\n\
         [[example]]\n\
         name = \"parse\"\n\
         path = \"examples/parse.rs\"\n\
         required-features = [\"tracing\"]\n\n\
         [[bench]]\n\
         name = \"parse\"\n\
         path = \"benches/parse.rs\"\n\
         harness = false\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src/syntax")).expect("create syntax");
    fs::create_dir_all(root.join("examples")).expect("create examples");
    fs::create_dir_all(root.join("benches")).expect("create benches");
    fs::write(root.join("src/lib.rs"), "pub mod syntax;\n").expect("write lib");
    fs::write(
        root.join("src/syntax/mod.rs"),
        "#[cfg(feature = \"tracing\")]\n\
         pub fn parse_document(input: &str) -> usize { tracing::trace!(input); input.len() }\n",
    )
    .expect("write syntax");
    fs::write(root.join("examples/parse.rs"), "fn main() {}\n").expect("write example");
    fs::write(root.join("benches/parse.rs"), "fn main() {}\n").expect("write bench");
}

fn write_member_package(
    root: &Path,
    relative: &str,
    package: &str,
    extra_manifest: &str,
    lib_source: &str,
) {
    let package_root = root.join(relative);
    fs::create_dir_all(package_root.join("src")).expect("create member src");
    fs::write(
        package_root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{extra_manifest}"
        ),
    )
    .expect("write member manifest");
    fs::write(package_root.join("src/lib.rs"), lib_source).expect("write member lib");
}
