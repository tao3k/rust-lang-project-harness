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
"###
    );
}

#[test]
fn cli_query_parser_code_preserves_whitespace_sensitive_literals() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_parser_literal_fixture(root);

    let output = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "items".as_ref(),
        "--query".as_ref(),
        "raw_indent|spaced_literal".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("r#\"\nalpha\n    beta\n\"#"), "{stdout}");
    assert!(stdout.contains("\"alpha    beta\""), "{stdout}");
    assert!(!stdout.contains("string[lines="), "{stdout}");
}

#[test]
fn cli_query_parser_json_marks_literal_compact_safety() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_parser_literal_fixture(root);

    let output = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--query".as_ref(),
        "raw_indent".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("query packet json");
    let projection = &value["matches"][0]["projection"];
    assert_eq!(projection["compactSafety"]["literalPolicy"], "summarize");
    assert_eq!(
        projection["compactSafety"]["whitespacePolicy"],
        "formatter-structural"
    );
    assert_eq!(projection["compactSafety"]["exactReadRequired"], true);
    assert_eq!(
        value["matches"][0]["patchSafety"]["level"],
        "ast-patch-safe"
    );
    assert_eq!(
        value["matches"][0]["patchSafety"]["target"]["read"],
        "src/lib.rs:1:6"
    );
    assert_eq!(
        value["matches"][0]["patchSafety"]["allowedOperations"],
        serde_json::json!(["replace_item", "split_owner_items"])
    );
    assert_eq!(
        value["matches"][0]["patchSafety"]["preimageSource"],
        "exact-read"
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
|query itemQuery=branch_and_write|match_and_loop status=hit match=exact item=2 reason=parser-item-exact next=query-code
|item branch_and_write kind=fn responsibilities=guard-branch,call-dispatch,early-return public=true next=symbol:branch_and_write read=src/lib.rs:3:10 syn=function_item/name
|item match_and_loop kind=fn responsibilities=state-mutation,bounded-loop,match-dispatch,match-arm,early-return public=true next=symbol:match_and_loop read=src/lib.rs:12:21 syn=function_item/name
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
    let match_value = &value["matches"][0];
    let projection = &match_value["projection"];

    assert_eq!(match_value["name"], "branch_and_write");
    assert_eq!(match_value["kind"], "fn");
    assert!(match_value["code"].is_null(), "{value}");
    assert!(
        projection["nodes"]
            .as_array()
            .is_some_and(|nodes| !nodes.is_empty()),
        "{value}"
    );
    assert!(
        projection["renderedRows"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty()),
        "{value}"
    );
    assert_eq!(projection["mode"], "compact");
    assert_eq!(projection["syntax"], "save-token-rustfmt");
    assert_eq!(projection["sourceAuthority"], "native-parser");
    assert_eq!(projection["losslessStructure"], true);
    assert_eq!(projection["exactRead"], "src/lib.rs:3:10");
}

#[test]
fn cli_query_parser_type_shape_includes_fields_and_impl_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_parser_data_shape_fixture(root);

    let output = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "items".as_ref(),
        "--query".as_ref(),
        "UserSummary".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    insta::assert_snapshot!(
        stdout.trim_end(),
        @r###"
pub struct UserSummary {
    pub user_id: u64,
    pub name: String,
    pub active: bool,
}
impl UserSummary {
    pub fn label(&self) -> String {
        if self.active {
            format!("{}#{}", self.name, self.user_id)
        } else {
            "inactive".to_string()
        }
    }
}
"###
    );
}

#[test]
fn cli_query_parser_type_shape_json_links_struct_and_impl_projection() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_parser_data_shape_fixture(root);

    let output = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--query".as_ref(),
        "UserSummary".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("query packet json");
    let matches = value["matches"].as_array().expect("matches");
    assert_eq!(matches.len(), 2, "{value}");
    assert_eq!(matches[0]["kind"], "struct");
    assert_eq!(matches[1]["kind"], "impl");
    for match_value in matches {
        let projection = &match_value["projection"];
        assert!(match_value["code"].is_null(), "{value}");
        assert!(
            projection["nodes"]
                .as_array()
                .is_some_and(|nodes| !nodes.is_empty()),
            "{value}"
        );
        assert!(
            projection["renderedRows"]
                .as_array()
                .is_some_and(|rows| !rows.is_empty()),
            "{value}"
        );
        assert_eq!(projection["mode"], "compact");
        assert_eq!(projection["sourceAuthority"], "native-parser");
        assert_eq!(projection["losslessStructure"], true);
        assert!(
            projection["exactRead"]
                .as_str()
                .is_some_and(|read| read.starts_with("src/lib.rs:")),
            "{value}"
        );
    }
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

fn write_parser_literal_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"literal-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        r###"pub fn raw_indent() -> &'static str {
    r#"
alpha
    beta
"#
}

pub fn spaced_literal() -> &'static str {
    "alpha    beta"
}
"###,
    )
    .expect("write source");
}

fn write_parser_data_shape_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"data-shape-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        r#"pub struct UserSummary {
    pub user_id: u64,
    pub name: String,
    pub active: bool,
}

impl UserSummary {
    pub fn label(&self) -> String {
        if self.active {
            format!("{}#{}", self.name, self.user_id)
        } else {
            "inactive".to_string()
        }
    }
}
"#,
    )
    .expect("write source");
}
