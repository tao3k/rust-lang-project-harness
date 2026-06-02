use std::fs;
use std::path::Path;

use tempfile::TempDir;

use crate::cli::support::{normalize_temp_root, run_cli};

#[test]
fn cli_query_parser_code_source_slice_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_parser_compact_fixture(root);

    let output = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "items".as_ref(),
        "--query".as_ref(),
        "branch_and_write|match_and_loop".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    insta::assert_snapshot!(
        stdout.trim_end(),
        @r###"
pub fn branch_and_write(flag: bool, block: &mut String) -> Option<String>
if let Some(line) = flag.then_some("ok")
call writeln!(block, "{line}")
return Some(line.to_string())
tail None
pub fn match_and_loop(values: &[String]) -> usize
let mut count = 0
for value in values
match value.as_str()
case "skip"
continue
case _
assign count += 1
tail count
"###
    );
}

#[test]
fn cli_query_parser_compact_line_protocol_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_parser_compact_fixture(root);

    let output = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "items".as_ref(),
        "--query".as_ref(),
        "branch_and_write|match_and_loop".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    );
    insta::assert_snapshot!(
        stdout.trim_end(),
        @r###"
[search-owner] q=src/lib.rs pkg=. own=1 item=2 itemQuery=branch_and_write|match_and_loop
|owner src/lib.rs role=source source=parser-visible-module lines=21 imports=1
|query itemQuery=branch_and_write|match_and_loop status=hit match=exact item=2 reason=parser-item-exact next=code
|item branch_and_write kind=fn public=true next=symbol:branch_and_write read=src/lib.rs:3:10
|code path=src/lib.rs lineRange=3:10 reason=item-query truncated=false nodes=n0:fn:declaration:0:3:10,n1:if:control-flow:1:4:7,n2:macro:call:2:5:5,n3:return:terminal:2:6:6,n4:tail:terminal:1:9:9 text="pub fn branch_and_write(flag: bool, block: &mut String) -> Option<String>\nif let Some(line) = flag.then_some(\"ok\")\ncall writeln!(block, \"{line}\")\nreturn Some(line.to_string())\ntail None"
|item match_and_loop kind=fn public=true next=symbol:match_and_loop read=src/lib.rs:12:21
|code path=src/lib.rs lineRange=12:21 reason=item-query truncated=false nodes=n0:fn:declaration:0:12:21,n1:let:mutation:1:13:13,n2:for:control-flow:1:14:19,n3:match:control-flow:2:15:18,n4:case:control-flow:3:16:16,n5:continue:terminal:4:16:16,n6:case:control-flow:3:17:17,n7:assign:mutation:4:17:17,n8:tail:terminal:1:20:20 text="pub fn match_and_loop(values: &[String]) -> usize\nlet mut count = 0\nfor value in values\nmatch value.as_str()\ncase \"skip\"\ncontinue\ncase _\nassign count += 1\ntail count"
"###
    );
}

#[test]
fn cli_query_parser_projection_nodes_feed_json_packet() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_parser_compact_fixture(root);

    let output = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--query".as_ref(),
        "branch_and_write".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("query packet json");
    let projection = &value["matches"][0]["projection"];

    assert_eq!(projection["nodes"][0]["id"], "n0");
    assert!(
        projection["nodes"][0]["label"]
            .as_str()
            .is_some_and(|label| label.starts_with("pub fn branch_and_write")),
        "{value}"
    );
    assert_eq!(projection["nodes"][1]["parentId"], "n0");
    assert_eq!(projection["nodes"][1]["read"], "src/lib.rs:4:7");
    assert_eq!(
        projection["nodes"][1]["flags"],
        serde_json::json!(["branch", "guard"])
    );
    assert_eq!(projection["expandActions"][0]["target"], "n1");
    assert_eq!(projection["expandActions"][0]["read"], "src/lib.rs:4:7");
}

fn write_parser_compact_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"compact-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        r#"use std::fmt::Write as _;

pub fn branch_and_write(flag: bool, block: &mut String) -> Option<String> {
    if let Some(line) = flag.then_some("ok") {
        let _ = writeln!(block, "{line}");
        return Some(line.to_string());
    }

    None
}

pub fn match_and_loop(values: &[String]) -> usize {
    let mut count = 0;
    for value in values {
        match value.as_str() {
            "skip" => continue,
            _ => count += 1,
        }
    }
    count
}
"#,
    )
    .expect("write source");
}
