use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::process::{Command, Output};

pub(crate) fn run_cli<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    Command::new(env!("CARGO_BIN_EXE_rs-harness"))
        .args(args)
        .output()
        .expect("run cli")
}

pub(crate) fn run_cli_with_env<I, S, K, V>(
    args: I,
    envs: impl IntoIterator<Item = (K, V)>,
) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
    K: AsRef<std::ffi::OsStr>,
    V: AsRef<std::ffi::OsStr>,
{
    Command::new(env!("CARGO_BIN_EXE_rs-harness"))
        .args(args)
        .envs(envs)
        .output()
        .expect("run cli")
}

pub(crate) fn run_cli_with_stdin<I, S>(args: I, stdin: &str) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut child = Command::new(env!("CARGO_BIN_EXE_rs-harness"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn cli");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("run cli")
}

#[cfg(feature = "search")]
pub(crate) fn run_search(root: &Path, args: &[&str]) -> String {
    let mut command_args = Vec::<std::ffi::OsString>::new();
    command_args.push("search".into());
    command_args.extend(args.iter().map(std::ffi::OsString::from));
    command_args.push(root.as_os_str().to_os_string());
    let output = run_cli(command_args);
    assert!(output.status.success(), "{output:?}");
    normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    )
}

#[cfg(feature = "search")]
pub(crate) fn run_search_with_stdin(root: &Path, args: &[&str], stdin: &str) -> String {
    let mut command_args = Vec::<std::ffi::OsString>::new();
    command_args.push("search".into());
    command_args.extend(args.iter().map(std::ffi::OsString::from));
    command_args.push(root.as_os_str().to_os_string());
    let output = run_cli_with_stdin(command_args, stdin);
    assert!(output.status.success(), "{output:?}");
    normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    )
}

pub(crate) fn write_manifest(root: &Path, name: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("write manifest");
}

#[cfg(feature = "search")]
pub(crate) fn write_search_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"cli-search-views\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [lib]\n\
         path = \"src/lib.rs\"\n\n\
         [[test]]\n\
         name = \"domain\"\n\
         path = \"tests/domain.rs\"\n\n\
         [features]\n\
         default = [\"serde\"]\n\
         json = [\"dep:serde\", \"serde/derive\"]\n\n\
         [dependencies]\n\
         anyhow = \"1\"\n\
         serde = { version = \"1\", optional = true, features = [\"derive\"] }\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::create_dir_all(root.join("tests")).expect("create tests");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n\
         mod domain;\n\
         use serde::Serialize;\n\
         pub use domain::Thing;\n\n\
         pub fn load() -> Thing { domain::make_thing() }\n\n\
         pub fn clone_value(input: String) -> String { input.clone() }\n\n\
         /// Fallible API.\n\
         pub async unsafe fn fallible(input: String) -> anyhow::Result<Thing> { let _ = input; todo!() }\n\n\
         pub struct PublicWire { pub serializer: serde::Serialize }\n\n\
         impl PublicWire { pub fn as_thing(&mut self) -> Thing { domain::make_thing() } }\n\n\
         pub trait WireApi { fn wire(&self) -> anyhow::Result<Thing>; }\n\
         impl WireApi for PublicWire { fn wire(&self) -> anyhow::Result<Thing> { todo!() } }\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/domain/mod.rs"),
        "//! Domain branch.\n\
         use serde::Serialize;\n\n\
         #[derive(Serialize)]\n\
         pub struct Thing { pub id: String }\n\n\
         pub fn make_thing() -> Thing { Thing { id: \"id\".to_string() } }\n\n\
         #[cfg(feature = \"json\")]\n\
         pub fn json_enabled() -> bool { true }\n",
    )
    .expect("write domain");
    fs::write(
        root.join("tests/domain.rs"),
        "use cli_search_views::load;\n\n\
         #[test]\n\
         fn loads() { let _ = load(); }\n",
    )
    .expect("write test");
}

#[cfg(feature = "search")]
pub(crate) fn write_complex_dependency_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"cli-complex-flow\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [lib]\n\
         path = \"src/lib.rs\"\n\n\
         [[test]]\n\
         name = \"flow\"\n\
         path = \"tests/flow.rs\"\n\n\
         [features]\n\
         default = [\"runtime\", \"walk\"]\n\
         runtime = [\"dep:tokio\", \"tokio/rt-multi-thread\", \"tokio/sync\", \"dep:bytes\"]\n\
         walk = [\"dep:ignore\"]\n\
         wire = [\"dep:bytes\"]\n\n\
         [dependencies]\n\
         bytes = { version = \"1\", optional = true }\n\
         ignore = { version = \"0.4\", optional = true }\n\
         tokio = { version = \"1\", optional = true, features = [\"rt-multi-thread\", \"sync\"] }\n\n\
         [lints.rust]\n\
         unexpected_cfgs = { level = \"warn\", check-cfg = ['cfg(tokio_unstable)'] }\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src/http")).expect("create http");
    fs::create_dir_all(root.join("src/io")).expect("create io");
    fs::create_dir_all(root.join("tests")).expect("create tests");
    fs::write(
        root.join("src/lib.rs"),
        "//! Complex dependency search fixture.\n\
         pub mod http;\n\
         pub mod io;\n\n\
         pub use http::client::{send_bytes, RuntimeApi, RuntimeClient};\n\
         pub use io::walk::{build_walk, WalkPlan};\n\n\
         #[cfg(feature = \"runtime\")]\n\
         pub fn runtime_enabled() -> bool { true }\n",
    )
    .expect("write lib");
    fs::write(root.join("src/http/mod.rs"), "pub mod client;\n").expect("write http mod");
    fs::write(
        root.join("src/http/client.rs"),
        "use bytes::{Buf, Bytes};\n\
         use tokio::sync::mpsc::Sender;\n\n\
         pub struct RuntimeClient { pub sender: Sender<Bytes>, pub buffer: Bytes }\n\n\
         pub async fn send_bytes(sender: Sender<Bytes>, mut payload: Bytes) -> Result<(), tokio::sync::mpsc::error::SendError<Bytes>> {\n\
         \tlet _ = payload.remaining();\n\
         \tsender.send(payload).await\n\
         }\n\n\
         pub trait RuntimeApi { fn sender(&self) -> Sender<Bytes>; }\n\n\
         impl RuntimeApi for RuntimeClient { fn sender(&self) -> Sender<Bytes> { self.sender.clone() } }\n\n\
         #[cfg(tokio_unstable)]\n\
         pub fn runtime_metrics() -> usize { 1 }\n",
    )
    .expect("write client");
    fs::write(root.join("src/io/mod.rs"), "pub mod walk;\n").expect("write io mod");
    fs::write(
        root.join("src/io/walk.rs"),
        "use bytes::Bytes;\n\
         use ignore::WalkBuilder;\n\n\
         pub struct WalkPlan { pub builder: WalkBuilder, pub seed: Bytes }\n\n\
         pub fn build_walk(root: &str) -> WalkBuilder { WalkBuilder::new(root) }\n\n\
         pub fn consume_chunk(chunk: Bytes) -> usize { chunk.len() }\n",
    )
    .expect("write walk");
    fs::write(
        root.join("tests/flow.rs"),
        "use cli_complex_flow::{build_walk, send_bytes, RuntimeClient, WalkPlan};\n\n\
         #[test]\n\
         fn routes_runtime_walk_and_bytes() { let _ = \"runtime tokio bytes ignore WalkBuilder RuntimeClient send_bytes WalkPlan build_walk\"; }\n",
    )
    .expect("write flow test");
}

pub(crate) fn write_clean_source(root: &Path) {
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
}

pub(crate) fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}
