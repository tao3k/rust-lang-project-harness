use std::fs;
use std::path::Path;

pub(super) fn write_public_tokio_bytes_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"tokio-public-io-lab\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [lib]\n\
         path = \"src/lib.rs\"\n\n\
         [[test]]\n\
         name = \"io_util\"\n\
         path = \"tests/io_util.rs\"\n\n\
         [features]\n\
         default = []\n\
         full = [\"io-util\", \"net\"]\n\
         io-util = [\"bytes\"]\n\
         net = []\n\n\
         [dependencies]\n\
         bytes = { version = \"1\", optional = true }\n",
    )
    .expect("write tokio manifest");
    fs::create_dir_all(root.join("src/io")).expect("create io");
    fs::create_dir_all(root.join("tests")).expect("create tests");
    fs::write(
        root.join("src/lib.rs"),
        "#[cfg(feature = \"io-util\")]\n\
         pub mod io;\n\
         #[cfg(feature = \"io-util\")]\n\
         pub use io::{read_buf, split_buf, AsyncReadExt, RuntimeFrame};\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/io/mod.rs"),
        "#![cfg(feature = \"io-util\")]\n\
         use bytes::{Buf, BufMut, Bytes, BytesMut};\n\n\
         pub struct RuntimeFrame { pub payload: Bytes, pub scratch: BytesMut }\n\n\
         pub trait AsyncReadExt { fn read_buf<B: BufMut>(&mut self, buf: &mut B) -> usize; }\n\n\
         pub fn read_buf<B: BufMut>(buf: &mut B) -> usize { buf.remaining_mut() }\n\n\
         pub fn split_buf(buffer: BytesMut) -> impl Buf { buffer.freeze() }\n",
    )
    .expect("write io");
    fs::write(
        root.join("tests/io_util.rs"),
        "use tokio_public_io_lab::{read_buf, RuntimeFrame};\n\n\
         #[test]\n\
         fn io_util_public_api_mentions_bytes() { let _ = \"io-util read_buf RuntimeFrame Buf BufMut\"; }\n",
    )
    .expect("write io util test");
}

pub(super) fn write_public_codex_web_search_workspace(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\n\
         members = [\n\
         \"crates/codex-api\",\n\
         \"crates/codex-protocol\",\n\
         \"ext/web-search\",\n\
         ]\n",
    )
    .expect("write codex workspace");
    write_package(
        root,
        "crates/codex-api",
        "codex-api",
        "",
        "pub enum SearchCommands { Search { query: String }, Open { url: String } }\n\
         pub struct SearchRequest { pub command: SearchCommands }\n",
    );
    write_package(
        root,
        "crates/codex-protocol",
        "codex-protocol",
        "",
        "pub enum WebSearchAction { Search { query: String }, Open { url: String } }\n\
         pub enum TurnItem { Started, Completed(WebSearchAction) }\n",
    );
    write_package(
        root,
        "ext/web-search",
        "codex-web-search",
        "[dependencies]\n\
         codex-api = { path = \"../../crates/codex-api\" }\n\
         codex-protocol = { path = \"../../crates/codex-protocol\" }\n",
        "pub mod tool;\n\
         pub use tool::{command_action, run_command, WebSearchTool};\n",
    );
    fs::write(
        root.join("ext/web-search/src/tool.rs"),
        "use codex_api::{SearchCommands, SearchRequest};\n\
         use codex_protocol::{TurnItem, WebSearchAction};\n\n\
         pub struct WebSearchTool { pub request: SearchRequest }\n\n\
         pub fn command_action(command: SearchCommands) -> WebSearchAction {\n\
             match command {\n\
                 SearchCommands::Search { query } => WebSearchAction::Search { query },\n\
                 SearchCommands::Open { url } => WebSearchAction::Open { url },\n\
             }\n\
         }\n\n\
         pub fn run_command(tool: WebSearchTool) -> Vec<TurnItem> {\n\
             vec![TurnItem::Started, TurnItem::Completed(command_action(tool.request.command))]\n\
         }\n",
    )
    .expect("write web search tool");
    fs::create_dir_all(root.join("ext/web-search/tests")).expect("create web search tests");
    fs::write(
        root.join("ext/web-search/tests/web_search.rs"),
        "use codex_web_search::{command_action, run_command, WebSearchTool};\n\n\
         #[test]\n\
         fn maps_search_command_action() { let _ = \"SearchCommands WebSearchAction command_action run_command WebSearchTool\"; }\n",
    )
    .expect("write web search test");
}

fn write_package(root: &Path, relative: &str, name: &str, extra_manifest: &str, lib_source: &str) {
    let package_root = root.join(relative);
    fs::create_dir_all(package_root.join("src")).expect("create package src");
    fs::write(
        package_root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{extra_manifest}"),
    )
    .expect("write package manifest");
    fs::write(package_root.join("src/lib.rs"), lib_source).expect("write package lib");
}
