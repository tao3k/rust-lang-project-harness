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
fn branch_and_write(flag: bool, block: &mut String) -> Option<String> {
    if let Some(line) = flag.then_some("ok") {
        writeln!(block, "{line}");
        return Some(line.to_string());
    }
    None
}
fn match_and_loop(values: &[String]) -> usize {
    let mut count = 0;
    for value in values {
        match value.as_str() {
            "skip" => {
                continue;
            }
            _ => {
                count += 1;
            }
        }
    }
    count
}
"###
    );
}

#[test]
fn cli_query_parser_code_summarizes_whitespace_sensitive_literals() {
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
    assert!(stdout.contains("raw-string[lines=4,bytes="), "{stdout}");
    assert!(stdout.contains("string[lines=1,bytes="), "{stdout}");
    assert!(!stdout.contains("alpha beta"), "{stdout}");
    assert!(!stdout.contains("alpha    beta"), "{stdout}");
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
    assert!(
        value["matches"][0]["code"]
            .as_str()
            .is_some_and(|code| code.contains("raw-string[lines=4,bytes=")),
        "{value}"
    );
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
        serde_json::json!(["replace_item"])
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
|query itemQuery=branch_and_write|match_and_loop status=hit match=exact item=2 reason=parser-item-exact next=code
|item branch_and_write kind=fn public=true next=symbol:branch_and_write read=src/lib.rs:3:10
|code path=src/lib.rs lineRange=3:10 reason=item-query truncated=false nodes=fn-3-10-7c6ac146:fn:declaration:0:3:10:rust-fn-3-10-7c6ac146:fn-declaration-3-10-7c6ac146,if-4-7-616cfb71:if:control-flow:1:4:7:rust-if-4-7-616cfb71:if-control-flow-4-7-616cfb71,macro-5-5-4a904212:macro:call:2:5:5:rust-macro-5-5-4a904212:macro-call-5-5-4a904212,return-6-6-7b0002eb:return:terminal:2:6:6:rust-return-6-6-7b0002eb:return-terminal-6-6-7b0002eb,return-9-9-304ff7fb:return:terminal:1:9:9:rust-return-9-9-304ff7fb:return-terminal-9-9-304ff7fb text="fn branch_and_write(flag: bool, block: &mut String) -> Option<String> {\n    if let Some(line) = flag.then_some(\"ok\") {\n        writeln!(block, \"{line}\");\n        return Some(line.to_string());\n    }\n    None\n}"
|item match_and_loop kind=fn public=true next=symbol:match_and_loop read=src/lib.rs:12:21
|code path=src/lib.rs lineRange=12:21 reason=item-query truncated=false nodes=fn-12-21-3e53336d:fn:declaration:0:12:21:rust-fn-12-21-3e53336d:fn-declaration-12-21-3e53336d,let-13-13-f426ba61:let:mutation:1:13:13:rust-let-13-13-f426ba61:let-mutation-13-13-f426ba61,for-14-19-24499ceb:for:control-flow:1:14:19:rust-for-14-19-24499ceb:for-control-flow-14-19-24499ceb,match-15-18-ba63a6a1:match:control-flow:2:15:18:rust-match-15-18-ba63a6a1:match-control-flow-15-18-ba63a6a1,case-16-16-1a012cee:case:control-flow:3:16:16:rust-case-16-16-1a012cee:case-control-flow-16-16-1a012cee,continue-16-16-d63d21ed:continue:terminal:4:16:16:rust-continue-16-16-d63d21ed:continue-terminal-16-16-d63d21ed,case-17-17-7c69d1e8:case:control-flow:3:17:17:rust-case-17-17-7c69d1e8:case-control-flow-17-17-7c69d1e8,assign-17-17-6c291f20:assign:mutation:4:17:17:rust-assign-17-17-6c291f20:assign-mutation-17-17-6c291f20,return-20-20-39b1ddf4:return:terminal:1:20:20:rust-return-20-20-39b1ddf4:return-terminal-20-20-39b1ddf4 text="fn match_and_loop(values: &[String]) -> usize {\n    let mut count = 0;\n    for value in values {\n        match value.as_str() {\n            \"skip\" => {\n                continue;\n            }\n            _ => {\n                count += 1;\n            }\n        }\n    }\n    count\n}"
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

    let nodes = projection["nodes"]
        .as_array()
        .expect("projection nodes array");
    let node_ids = nodes
        .iter()
        .map(|node| node["id"].as_str().expect("node id"))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(node_ids.len(), nodes.len(), "{value}");
    assert!(
        nodes[0]["label"]
            .as_str()
            .is_some_and(|label| label.starts_with("fn branch_and_write")),
        "{value}"
    );
    assert_eq!(nodes[1]["parentId"], nodes[0]["id"]);
    assert_eq!(nodes[1]["read"], "src/lib.rs:4:7");
    assert!(
        nodes[0]["nativeId"]
            .as_str()
            .is_some_and(|id| id.starts_with("rust-fn-"))
    );
    assert!(nodes[1]["structuralFingerprint"].as_str().is_some());
    assert_eq!(nodes[1]["flags"], serde_json::json!(["branch", "guard"]));
    let rendered_node_ids = projection["renderedNodeIds"]
        .as_array()
        .expect("rendered node ids");
    assert_eq!(rendered_node_ids.len(), nodes.len(), "{value}");
    for node_id in rendered_node_ids {
        assert!(
            node_ids.contains(node_id.as_str().expect("rendered node id")),
            "{value}"
        );
    }
    assert_eq!(projection["omitted"][0]["nodeId"], nodes[0]["id"]);
    let expand_target = projection["expandActions"][0]["target"]
        .as_str()
        .expect("expand target");
    assert!(node_ids.contains(expand_target), "{value}");
    assert_eq!(projection["expandActions"][0]["read"], "src/lib.rs:4:7");
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
    fn label(&self) -> String {
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
    assert!(
        matches[0]["projection"]["nodes"]
            .as_array()
            .expect("struct nodes")
            .iter()
            .any(|node| node["kind"] == "field" && node["label"] == "pub user_id: u64,"),
        "{value}"
    );
    assert_eq!(matches[0]["projection"]["sourceAuthority"], "native-parser");
    assert_eq!(matches[0]["projection"]["losslessStructure"], true);
    assert!(
        matches[1]["projection"]["nodes"]
            .as_array()
            .expect("impl nodes")
            .iter()
            .any(|node| node["kind"] == "fn"
                && node["label"]
                    .as_str()
                    .is_some_and(|label| label.starts_with("fn label"))),
        "{value}"
    );
    assert!(
        matches[1]["projection"]["expandActions"]
            .as_array()
            .expect("expand actions")
            .iter()
            .any(|action| action["reason"] == "parser-projection-control-flow"),
        "{value}"
    );
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
