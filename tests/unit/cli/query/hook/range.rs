use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use crate::cli::support::{run_cli, write_search_fixture};

#[test]
fn cli_query_hook_line_range_code_outputs_local_window() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"query-range\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write manifest");
    fs::write(
        root.join("src/lib.rs"),
        r#"fn run_facade(language: &str) -> Output {
    todo!()
}

mod language {
    use super::run_facade;

    #[test]
    fn rust_facade_invokes_provider_query() {
        let output = run_facade("rust");
        assert!(output.status.success());
    }
}
"#,
    )
    .expect("write lib");
    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:5:11".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(!stdout.contains("[search-owner]"), "{stdout}");
    assert!(!stdout.contains("[read-plan]"), "{stdout}");
    assert!(stdout.contains("mod language {"), "{stdout}");
    assert!(stdout.contains("    use super::run_facade;"), "{stdout}");
    assert!(
        stdout.contains("    fn rust_facade_invokes_provider_query() {"),
        "{stdout}"
    );
    assert!(
        stdout.contains("        assert!(output.status.success());"),
        "{stdout}"
    );
    assert!(stdout.lines().any(|line| line == "}"), "{stdout}");
    assert!(stdout.lines().count() > 1, "{stdout}");
}

#[test]
fn cli_query_hook_line_range_code_uses_projection_rows_for_nested_impl() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"query-range-impl\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write manifest");
    fs::write(
        root.join("src/lib.rs"),
        r#"struct LocalNativeCliBackend;
struct LocalNativeCommand {
    program: String,
    args: Vec<String>,
}

impl LocalNativeCliBackend {
}

impl LocalNativeCommand {
    fn argv(&self) -> Vec<String> {
        let mut argv = Vec::with_capacity(self.args.len() + 1);
        argv.push(self.program.clone());
        argv.extend(self.args.clone());
        argv
    }
}
"#,
    )
    .expect("write lib");
    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:7:18".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(!stdout.contains("[search-owner]"), "{stdout}");
    assert!(!stdout.contains("[read-plan]"), "{stdout}");
    assert_no_punctuation_only_lines(&stdout);
    insta::assert_snapshot!(stdout.trim_end(), @r#"
impl LocalNativeCliBackend {
}
impl LocalNativeCommand {
    fn argv(&self) -> Vec<String> {
        let mut argv = Vec::with_capacity(self.args.len() + 1);
        argv.push(self.program.clone())
        argv.extend(self.args.clone())
        argv
    }
}
"#);
}

#[test]
fn cli_query_hook_wide_line_range_code_returns_read_plan_without_source() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);
    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:1:80".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.starts_with("[read-plan] "), "{stdout}");
    assert!(stdout.contains("mode=range-frontier"), "{stdout}");
    assert!(stdout.contains("code=false"), "{stdout}");
    assert!(stdout.contains("reason=wide-selector"), "{stdout}");
    assert!(stdout.contains("maxWindow=40"), "{stdout}");
    assert!(stdout.contains("alg=symbol-frontier"), "{stdout}");
    assert!(stdout.contains("requested=1:80"), "{stdout}");
    assert!(
        stdout.contains("S=symbol:mod(domain)@src/lib.rs:2:2!code"),
        "{stdout}"
    );
    assert!(
        stdout.contains("S2=symbol:fn(load)@src/lib.rs:6:6!code"),
        "{stdout}"
    );
    assert!(stdout.contains("rank=S,S2"), "{stdout}");
    assert!(stdout.contains("frontier=S.code,S2.code"), "{stdout}");
    assert!(
        stdout.contains("avoid=repeat-wide-read,manual-window-scan,raw-read"),
        "{stdout}"
    );
    assert!(stdout.contains("|symbol item=domain kind=mod lineRange=2:2 read=src/lib.rs:2:2 lineCount=1 reason=parser-item"), "{stdout}");
    assert!(stdout.contains("|symbol item=load kind=fn lineRange=6:6 read=src/lib.rs:6:6 lineCount=1 reason=parser-item"), "{stdout}");
    assert!(!stdout.contains("|symbol item=local-window"), "{stdout}");
    assert!(!stdout.contains("pub fn load()"), "{stdout}");
    assert!(!stdout.contains("[search-owner]"), "{stdout}");
}

#[test]
fn cli_query_hook_wide_line_range_json_returns_symbol_read_plan_packet() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:1:80".as_ref(),
        "--code".as_ref(),
        "--view".as_ref(),
        "read-packet".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let value = serde_json::from_slice::<Value>(&output.stdout).expect("read json");
    assert_eq!(value["schemaVersion"], "1");
    assert_eq!(value["selector"], "src/lib.rs:1:80");
    assert!(value.get("sourceWindows").is_none(), "{value}");

    let plan = &value["readPlan"];
    assert_eq!(plan["mode"], "range-frontier");
    assert_eq!(plan["code"], false);
    assert_eq!(plan["reason"], "wide-selector");
    assert_eq!(plan["algorithm"], "symbol-frontier");
    assert_eq!(plan["maxWindowLines"], 40);
    assert!(plan["windows"].as_array().is_none(), "{value}");

    let symbols = plan["symbols"].as_array().expect("symbols");
    assert!(
        symbols.iter().any(|symbol| symbol["itemName"] == "domain"
            && symbol["itemKind"] == "mod"
            && symbol["read"] == "src/lib.rs:2:2"),
        "{value}"
    );
    assert!(
        symbols.iter().any(|symbol| symbol["itemName"] == "load"
            && symbol["itemKind"] == "fn"
            && symbol["read"] == "src/lib.rs:6:6"),
        "{value}"
    );

    let frontier = plan["frontier"].as_array().expect("frontier");
    assert_eq!(frontier[0]["id"], "S");
    assert_eq!(frontier[0]["kind"], "symbol");
    assert_eq!(frontier[0]["action"], "code");
    assert_eq!(frontier[0]["read"], "src/lib.rs:2:2");
    assert_eq!(frontier[1]["id"], "S2");
    assert_eq!(frontier[1]["kind"], "symbol");
    assert_eq!(frontier[1]["action"], "code");
    assert_eq!(frontier[1]["read"], "src/lib.rs:6:6");
}

#[test]
fn cli_query_hook_wide_line_range_without_parser_items_returns_window_frontier() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"invalid-range\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write manifest");
    let mut source = String::from("pub fn broken(\n");
    for line in 2..=80 {
        source.push_str(&format!("// line {line}\n"));
    }
    fs::write(root.join("src/lib.rs"), source).expect("write source");

    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:1:80".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.starts_with("[read-plan] "), "{stdout}");
    assert!(stdout.contains("alg=range-split"), "{stdout}");
    assert!(
        stdout.contains("W=window:range(src/lib.rs@1:40)!code"),
        "{stdout}"
    );
    assert!(
        stdout.contains("W2=window:range(src/lib.rs@41:80)!code"),
        "{stdout}"
    );
    assert!(stdout.contains("rank=W,W2"), "{stdout}");
    assert!(stdout.contains("frontier=W.code,W2.code"), "{stdout}");
    assert!(
        stdout.contains(
            "|window path=src/lib.rs lineRange=1:40 read=src/lib.rs:1:40 lineCount=40 reason=split"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("|window path=src/lib.rs lineRange=41:80 read=src/lib.rs:41:80 lineCount=40 reason=split"), "{stdout}");
    assert!(!stdout.contains("|symbol item=local-window"), "{stdout}");
}

#[test]
fn cli_query_hook_wide_line_range_json_falls_back_to_window_read_plan_packet() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"invalid-range-json\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write manifest");
    let mut source = String::from("pub fn broken(\n");
    for line in 2..=80 {
        source.push_str(&format!("// line {line}\n"));
    }
    fs::write(root.join("src/lib.rs"), source).expect("write source");

    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:1:80".as_ref(),
        "--code".as_ref(),
        "--view".as_ref(),
        "read-packet".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let value = serde_json::from_slice::<Value>(&output.stdout).expect("read json");
    assert_eq!(value["schemaVersion"], "1");
    assert_eq!(value["selector"], "src/lib.rs:1:80");
    assert!(value.get("sourceWindows").is_none(), "{value}");

    let plan = &value["readPlan"];
    assert_eq!(plan["mode"], "range-frontier");
    assert_eq!(plan["algorithm"], "range-split");
    assert!(plan["symbols"].as_array().is_none(), "{value}");

    let windows = plan["windows"].as_array().expect("windows");
    assert_eq!(windows.len(), 2, "{value}");
    assert_eq!(windows[0]["read"], "src/lib.rs:1:40");
    assert_eq!(windows[1]["read"], "src/lib.rs:41:80");

    let frontier = plan["frontier"].as_array().expect("frontier");
    assert_eq!(frontier[0]["id"], "W");
    assert_eq!(frontier[0]["kind"], "window");
    assert_eq!(frontier[0]["read"], "src/lib.rs:1:40");
    assert_eq!(frontier[1]["id"], "W2");
    assert_eq!(frontier[1]["kind"], "window");
    assert_eq!(frontier[1]["read"], "src/lib.rs:41:80");
}

fn assert_no_punctuation_only_lines(stdout: &str) {
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "}" {
            continue;
        }
        assert!(
            trimmed.chars().any(|ch| ch.is_alphanumeric() || ch == '_'),
            "punctuation-only compact row leaked: {stdout}"
        );
    }
}
