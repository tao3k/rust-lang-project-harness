use std::fmt::Write as _;
use std::fs;
use std::path::Path;

pub(super) fn write_tokio_io_uring_fixture(root: &Path) {
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

pub(super) fn write_large_rustc_workspace_fixture(root: &Path) {
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

pub(super) fn write_file_search_ignore_fixture(root: &Path) {
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

pub(super) fn write_orgize_tracing_fixture(root: &Path) {
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
