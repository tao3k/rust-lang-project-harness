use tempfile::TempDir;

use crate::cli::support::run_cli;

fn write_flow_lite_fixture(root: &std::path::Path) {
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        r#"struct ToolAction {
    payload: String,
}

fn payload_string(input: &str) -> String {
    input.to_string()
}

fn collect_tool_actions(input: &str) -> Vec<ToolAction> {
    let payload = payload_string(input);
    vec![ToolAction { payload }]
}
"#,
    )
    .expect("fixture");
}

fn write_large_flow_lite_fixture(root: &std::path::Path) {
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    for index in 0..70 {
        std::fs::write(
            root.join("src").join(format!("mod_{index}.rs")),
            format!("fn unrelated_{index}() {{}}\n"),
        )
        .expect("fixture file");
    }
}

#[test]
fn flow_lite_query_catalog_outputs_compact_locator_frontier() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_flow_lite_fixture(root);

    let output = run_cli([
        "query",
        "--catalog",
        "flow-lite",
        "--where",
        "source.call=payload_string sink.constructs=ToolAction scope.fn=collect_tool_actions",
        root.to_str().expect("root utf8"),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.starts_with("[query-flow-lite] root="), "{stdout}");
    assert!(
        stdout.contains("catalog=flow-lite flow=local-source-sink"),
        "{stdout}"
    );
    assert!(
        stdout.contains("scope=fn(collect_tool_actions)"),
        "{stdout}"
    );
    assert!(
        stdout.contains("S=source:call(payload_string)@src/lib.rs:10!code"),
        "{stdout}"
    );
    assert!(
        stdout.contains("K=sink:constructs(ToolAction)@src/lib.rs:11!code"),
        "{stdout}"
    );
    assert!(stdout.contains("P=path:bounded(S->K)!flow"), "{stdout}");
    assert!(stdout.contains("S>{K:flows-to}"), "{stdout}");
    assert!(stdout.contains("confidence=bounded sourceAuthority=native-parser executionBackend=native-parser adapterMode=native-projection"), "{stdout}");
    assert!(stdout.contains("frontier=S.code,K.code"), "{stdout}");
    assert!(
        stdout.contains("avoid=codeql-hot-path,raw-read,inline-code"),
        "{stdout}"
    );
    assert!(!stdout.contains("let payload = payload_string(input);"));
    assert!(!stdout.contains("ToolAction { payload }"));
    assert!(!stdout.contains("codeql database"));
}

#[test]
fn flow_lite_query_catalog_outputs_semantic_flow_lite_json() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_flow_lite_fixture(root);

    let output = run_cli([
        "query",
        "--catalog",
        "flow-lite",
        "--where",
        "source.call=payload_string sink.constructs=ToolAction scope.fn=collect_tool_actions",
        "--json",
        root.to_str().expect("root utf8"),
    ]);
    assert!(output.status.success(), "{output:?}");

    let json = serde_json::from_slice::<serde_json::Value>(&output.stdout).expect("flow-lite json");
    assert_eq!(
        json["schemaId"],
        "agent.semantic-protocols.semantic-flow-lite"
    );
    assert_eq!(json["flowKind"], "local-source-sink");
    assert_eq!(json["sourceAuthority"], "native-parser");
    assert_eq!(json["executionBackend"], "native-parser");
    assert_eq!(json["adapterMode"], "native-projection");
    assert_eq!(json["confidence"], "bounded");
    assert_eq!(json["ownerPath"], "src/lib.rs");
    assert_eq!(json["path"].as_array().expect("path").len(), 3);
    assert_eq!(json["path"][0]["relation"], "source");
    assert_eq!(json["path"][1]["relation"], "sink");
    assert_eq!(json["path"][2]["relation"], "flows-to");
    assert_eq!(json["fields"]["catalog"], "flow-lite");
    assert_eq!(json["fields"]["rawSourceStored"], false);
    assert_eq!(json["omissions"].as_array().expect("omissions").len(), 0);
}

#[test]
fn flow_lite_query_catalog_requires_owner_for_large_projects() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_large_flow_lite_fixture(root);

    let output = run_cli([
        "query",
        "--catalog",
        "flow-lite",
        "--where",
        "source.call=payload sink.constructs=Action scope.fn=collect",
        root.to_str().expect("root utf8"),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("confidence=unavailable"), "{stdout}");
    assert!(stdout.contains("scannedFiles=70"), "{stdout}");
    assert!(stdout.contains("reason=scope-not-narrowed"), "{stdout}");
    assert!(
        stdout.contains("avoid=codeql-hot-path,raw-read,inline-code"),
        "{stdout}"
    );
}

#[test]
fn flow_lite_query_catalog_accepts_explicit_owner_path() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_flow_lite_fixture(root);

    let output = run_cli([
        "query",
        "--catalog",
        "flow-lite",
        "--where",
        "source.call=payload_string sink.constructs=ToolAction scope.fn=collect_tool_actions owner.path=src/lib.rs",
        root.to_str().expect("root utf8"),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("confidence=bounded"), "{stdout}");
    assert!(stdout.contains("scannedFiles=1"), "{stdout}");
    assert!(
        stdout.contains("S=source:call(payload_string)@src/lib.rs:10!code"),
        "{stdout}"
    );
}

#[test]
fn flow_lite_query_catalog_rejects_open_where_language() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_flow_lite_fixture(root);

    let output = run_cli([
        "query",
        "--catalog",
        "flow-lite",
        "--where",
        "source.call=payload_string sink.constructs=ToolAction scope.fn=collect_tool_actions guard.eq=is_safe",
        root.to_str().expect("root utf8"),
    ]);
    assert!(!output.status.success(), "{output:?}");

    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(
        stderr.contains("unsupported flow-lite --where key `guard.eq`"),
        "{stderr}"
    );
}
