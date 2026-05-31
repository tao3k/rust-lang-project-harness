use std::fs;
#[cfg(feature = "search")]
use std::io::Write;
use std::path::Path;
#[cfg(feature = "search")]
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
    let mut child = Command::new(env!("CARGO_BIN_EXE_rs-harness"))
        .args(command_args)
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
    let output = child.wait_with_output().expect("run cli");
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
