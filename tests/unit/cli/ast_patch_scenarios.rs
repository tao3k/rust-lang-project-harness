use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::support::{run_cli, run_cli_with_stdin};

#[test]
fn cli_ast_patch_scenarios_match_expected_trees_and_receipts() {
    if crate::cli::support::skip_if_protocol_graph_renderer_unavailable() {
        return;
    }

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("ast_patch_scenarios");
    let mut scenarios = fs::read_dir(&fixtures)
        .unwrap_or_else(|error| panic!("read fixtures {}: {error}", fixtures.display()))
        .map(|entry| entry.expect("fixture entry").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    scenarios.sort();
    assert!(
        !scenarios.is_empty(),
        "expected ast patch scenarios in {}",
        fixtures.display()
    );
    for scenario in scenarios {
        run_ast_patch_scenario(&scenario);
    }

    let provider_schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("schemas")
        .join("rust-ast-patch-real-project-evidence.v1.schema.json");
    let root_schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| panic!("workspace root for {}", env!("CARGO_MANIFEST_DIR")))
        .join("schemas")
        .join("rust-ast-patch-real-project-evidence.v1.schema.json");
    let provider_schema = fs::read_to_string(&provider_schema_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", provider_schema_path.display()));
    if root_schema_path.exists() {
        let root_schema = fs::read_to_string(&root_schema_path)
            .unwrap_or_else(|error| panic!("read {}: {error}", root_schema_path.display()));
        assert_eq!(
            provider_schema, root_schema,
            "Rust evidence schema copy drift"
        );
    }
    let evidence_schema = serde_json::from_str::<Value>(&provider_schema)
        .unwrap_or_else(|error| panic!("parse {}: {error}", provider_schema_path.display()));
    assert_eq!(
        evidence_schema["properties"]["schemaId"]["const"],
        "agent.semantic-protocols.rust-ast-patch-real-project-evidence"
    );

    let evidence_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("ast_patch_real_projects");
    let mut evidence_files = fs::read_dir(&evidence_dir)
        .unwrap_or_else(|error| {
            panic!("read evidence fixtures {}: {error}", evidence_dir.display())
        })
        .map(|entry| entry.expect("evidence entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    evidence_files.sort();
    assert!(
        !evidence_files.is_empty(),
        "expected real-project ast patch evidence fixtures in {}",
        evidence_dir.display()
    );
    let evidence_has_event = |receipt: &Value, expected: &str| {
        receipt["verificationEvents"]
            .as_array()
            .unwrap_or_else(|| {
                panic!("real-project evidence receipt events must be an array: {receipt}")
            })
            .iter()
            .any(|event| event.as_str() == Some(expected))
    };

    for evidence_file in evidence_files {
        let evidence = read_json(&evidence_file);
        let label = evidence_file.display();
        assert_eq!(
            evidence["schemaId"], "agent.semantic-protocols.rust-ast-patch-real-project-evidence",
            "{label}"
        );
        assert_eq!(evidence["schemaVersion"], "1", "{label}");
        assert_eq!(evidence["sourceStored"], false, "{label}");
        assert_eq!(
            evidence["selectedMatch"]["patchSafetyLevel"], "ast-patch-safe",
            "{label}"
        );
        assert_eq!(
            evidence["selectedMatch"]["allowedOperations"],
            json!(["replace_item"]),
            "{label}"
        );
        assert_eq!(
            evidence["compact"]["syntax"], "save-token-rustfmt",
            "{label}"
        );
        assert_eq!(
            evidence["compact"]["whitespacePolicy"], "formatter-structural",
            "{label}"
        );
        assert_eq!(evidence["compact"]["exactReadRequired"], true, "{label}");
        let compact_bytes = evidence["compact"]["compactBytes"]
            .as_u64()
            .unwrap_or_else(|| panic!("{label}: compactBytes must be numeric"));
        let exact_bytes = evidence["compact"]["exactBytes"]
            .as_u64()
            .unwrap_or_else(|| panic!("{label}: exactBytes must be numeric"));
        assert!(
            compact_bytes < exact_bytes,
            "{label}: compact projection must save tokens: compact={compact_bytes} exact={exact_bytes}"
        );
        assert!(
            evidence["compact"]["compactToExactRatio"]
                .as_f64()
                .unwrap_or(1.0)
                < 1.0,
            "{label}: compact ratio must be less than exact source"
        );
        assert!(
            evidence["compact"]["responsibilities"]
                .as_array()
                .is_some_and(|responsibilities| responsibilities.len() >= 3),
            "{label}: real-project evidence needs multiple parser-owned responsibilities"
        );
        assert_eq!(evidence["dryRunReceipt"]["status"], "verified", "{label}");
        assert_eq!(
            evidence["dryRunReceipt"]["capability"], "provider-ast-dry-run",
            "{label}"
        );
        assert_eq!(
            evidence["dryRunReceipt"]["mutationAvailable"], false,
            "{label}"
        );
        assert!(
            evidence_has_event(&evidence["dryRunReceipt"], "file-reparsed"),
            "{label}: dry-run receipt must reparse file"
        );
        assert!(
            !evidence_has_event(&evidence["dryRunReceipt"], "file-written"),
            "{label}: dry-run receipt must not write files"
        );
        assert_eq!(evidence["applyTempReceipt"]["status"], "applied", "{label}");
        assert_eq!(
            evidence["applyTempReceipt"]["capability"], "provider-ast-apply",
            "{label}"
        );
        assert_eq!(
            evidence["applyTempReceipt"]["mutationAvailable"], true,
            "{label}"
        );
        assert!(
            evidence_has_event(&evidence["applyTempReceipt"], "formatter-output-reparsed"),
            "{label}: apply receipt must reparse rustfmt output"
        );
        assert!(
            evidence_has_event(&evidence["applyTempReceipt"], "file-written"),
            "{label}: temp apply receipt must write the temp file"
        );
    }
}

#[test]
fn cli_ast_patch_scenario_uses_query_patch_safety_target_for_tokio_style_apply() {
    let fixture = ast_patch_scenarios_dir().join("009_tokio_style_nested_async_fn_apply");
    let temp = tempfile::tempdir().expect("tempdir");
    copy_dir_recursive(&fixture.join("input"), temp.path());

    let query = run_cli([
        "query".as_ref(),
        "src/runtime/scheduler/multi_thread/worker.rs".as_ref(),
        "--query".as_ref(),
        "park_timeout".as_ref(),
        "--json".as_ref(),
        temp.path().as_os_str(),
    ]);
    assert!(query.status.success(), "{query:?}");
    let query_packet = serde_json::from_slice::<Value>(&query.stdout).expect("query packet");
    let match_value = &query_packet["matches"][0];
    assert_eq!(
        match_value["patchSafety"]["level"], "ast-patch-safe",
        "{query_packet}"
    );
    assert!(
        match_value["patchSafety"]["allowedOperations"]
            .as_array()
            .is_some_and(|operations| operations
                .iter()
                .any(|operation| operation == "replace_item")),
        "{query_packet}"
    );
    assert_eq!(
        match_value["projection"]["compactSafety"]["exactReadRequired"], true,
        "{query_packet}"
    );

    let target = match_value["patchSafety"]["target"].clone();
    let preimage = exact_read_from_target(temp.path(), &target);
    assert!(preimage.contains("pub(crate) async fn park_timeout"));
    let fixture_packet = read_json(&fixture.join("packet.json"));
    let snippet = fixture_packet["operation"]["snippet"]
        .as_str()
        .expect("fixture replacement snippet");
    let packet = json!({
        "target": target,
        "operation": {
            "op": "replace_item",
            "snippet": snippet,
            "expectedSnippet": preimage,
            "maxEdits": 1
        }
    })
    .to_string();

    let output = run_cli_with_stdin(
        [
            OsString::from("ast-patch"),
            OsString::from("apply"),
            OsString::from("--packet"),
            OsString::from("-"),
            temp.path().as_os_str().to_os_string(),
        ],
        &packet,
    );
    assert!(output.status.success(), "{output:?}");
    assert!(
        output.stdout.is_empty(),
        "successful ast-patch apply should not print a receipt: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let expected = snapshot_dir(&fixture.join("expected"));
    let actual = snapshot_dir(temp.path());
    assert_eq!(
        actual, expected,
        "query-derived ast-patch target should apply cleanly"
    );
}

#[test]
#[ignore = "set ASP_AST_PATCH_REAL_ROOT/PATH/QUERY to exercise a real Rust checkout"]
fn cli_ast_patch_real_checkout_query_target_dry_runs_from_env() {
    let Some(root) = env_path("ASP_AST_PATCH_REAL_ROOT") else {
        eprintln!(
            "skipping real checkout ast-patch evidence; set ASP_AST_PATCH_REAL_ROOT, ASP_AST_PATCH_REAL_PATH, and ASP_AST_PATCH_REAL_QUERY"
        );
        return;
    };
    let Some(source_path) = env_string("ASP_AST_PATCH_REAL_PATH") else {
        eprintln!("skipping real checkout ast-patch evidence; ASP_AST_PATCH_REAL_PATH is unset");
        return;
    };
    let Some(query_term) = env_string("ASP_AST_PATCH_REAL_QUERY") else {
        eprintln!("skipping real checkout ast-patch evidence; ASP_AST_PATCH_REAL_QUERY is unset");
        return;
    };

    let query = run_cli([
        "query".as_ref(),
        source_path.as_ref(),
        "--query".as_ref(),
        query_term.as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(query.status.success(), "{query:?}");
    let query_packet = serde_json::from_slice::<Value>(&query.stdout).expect("query packet JSON");
    let target_kind = env_string("ASP_AST_PATCH_REAL_TARGET_KIND");
    let target_name = env_string("ASP_AST_PATCH_REAL_TARGET_NAME");
    let matches = query_packet["matches"].as_array().unwrap_or_else(|| {
        panic!("real checkout query packet matches must be an array: {query_packet}")
    });
    let match_value = matches
        .iter()
        .find(|match_value| {
            match_value["patchSafety"]["level"] == "ast-patch-safe"
                && target_kind.as_deref().is_none_or(|kind| {
                    match_value["kind"].as_str() == Some(kind)
                })
                && target_name.as_deref().is_none_or(|name| {
                    match_value["name"].as_str() == Some(name)
                })
        })
        .unwrap_or_else(|| {
            panic!(
                "real checkout query packet had no ast-patch-safe match kind={target_kind:?} name={target_name:?}: {query_packet}"
            )
        });
    assert_eq!(
        match_value["patchSafety"]["allowedOperations"],
        json!(["replace_item"]),
        "{query_packet}"
    );

    let target = match_value["patchSafety"]["target"].clone();
    let preimage = exact_read_from_target(&root, &target);
    assert!(!preimage.trim().is_empty(), "empty preimage from {target}");
    let packet = json!({
        "target": target,
        "operation": {
            "op": "replace_item",
            "snippet": preimage,
            "expectedSnippet": preimage,
            "maxEdits": 1
        }
    })
    .to_string();

    let dry_run = run_cli_with_stdin(
        [
            OsString::from("ast-patch"),
            OsString::from("dry-run"),
            OsString::from("--packet"),
            OsString::from("-"),
            root.as_os_str().to_os_string(),
        ],
        &packet,
    );
    assert!(dry_run.status.success(), "{dry_run:?}");
    let receipt = serde_json::from_slice::<Value>(&dry_run.stdout).expect("dry-run receipt JSON");
    assert_eq!(receipt["status"], "verified", "{receipt}");
    assert_receipt_verification_contains("real_checkout_dry_run", &receipt, "file-reparsed");

    if env_flag("ASP_AST_PATCH_REAL_APPLY_TEMP") {
        let temp = tempfile::tempdir().expect("tempdir");
        copy_target_file_to_temp(&root, temp.path(), &source_path);
        let apply = run_cli_with_stdin(
            [
                OsString::from("ast-patch"),
                OsString::from("apply"),
                OsString::from("--packet"),
                OsString::from("-"),
                temp.path().as_os_str().to_os_string(),
            ],
            &packet,
        );
        assert!(apply.status.success(), "{apply:?}");
        let receipt = serde_json::from_slice::<Value>(&apply.stdout).expect("apply receipt JSON");
        assert_eq!(receipt["status"], "applied", "{receipt}");
        assert_receipt_verification_contains(
            "real_checkout_apply_temp",
            &receipt,
            "formatter-output-reparsed",
        );
    }
}

fn run_ast_patch_scenario(dir: &Path) {
    let name = scenario_name(dir);
    let temp = tempfile::tempdir()
        .unwrap_or_else(|error| panic!("{name}: failed to create tempdir: {error}"));
    copy_dir_recursive(&dir.join("input"), temp.path());

    let scenario = read_json(&dir.join("scenario.json"));
    let mode = scenario["mode"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: scenario mode must be a string"));
    let expected_status = scenario["expectedStatus"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: expectedStatus must be a string"));
    let packet = fs::read_to_string(dir.join("packet.json"))
        .unwrap_or_else(|error| panic!("{name}: failed to read packet: {error}"));

    let output = run_cli_with_stdin(
        [
            OsString::from("ast-patch"),
            OsString::from(mode),
            OsString::from("--packet"),
            OsString::from("-"),
            temp.path().as_os_str().to_os_string(),
        ],
        &packet,
    );
    let status = output.status;
    let stdout = String::from_utf8(output.stdout)
        .unwrap_or_else(|error| panic!("{name}: stdout was not utf-8: {error}"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        status.success(),
        "{name}: ast-patch command failed: status={status:?} stdout={stdout} stderr={stderr}"
    );

    if mode == "apply" && expected_status == "applied" {
        assert!(
            stdout.is_empty(),
            "{name}: successful ast-patch apply should not print a receipt: {stdout}"
        );
    } else {
        let receipt = serde_json::from_str::<Value>(&stdout)
            .unwrap_or_else(|error| panic!("{name}: receipt JSON: {error}: {stdout}"));
        assert_receipt_matches(&name, &scenario, &receipt);
    }

    let expected = snapshot_dir(&dir.join("expected"));
    let actual = snapshot_dir(temp.path());
    assert_eq!(
        actual, expected,
        "{name}: applied tree should match expected fixture"
    );

    for compact_check in json_array(&scenario, "compactChecks") {
        assert_compact_check(&name, dir, temp.path(), compact_check);
    }
}

fn ast_patch_scenarios_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("ast_patch_scenarios")
}

fn assert_receipt_matches(name: &str, scenario: &Value, receipt: &Value) {
    let expected_status = scenario["expectedStatus"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: scenario.expectedStatus must be a string"));
    assert_eq!(
        receipt["status"].as_str(),
        Some(expected_status),
        "{name}: receipt.status"
    );
    assert_eq!(
        receipt["mode"].as_str(),
        scenario["mode"].as_str(),
        "{name}: receipt.mode"
    );

    if let Some(expected) = scenario.get("expectedCapability").and_then(Value::as_str) {
        assert_eq!(
            receipt["capability"].as_str(),
            Some(expected),
            "{name}: receipt.capability"
        );
    }
    if let Some(expected) = scenario
        .get("expectedMutationAvailable")
        .and_then(Value::as_bool)
    {
        assert_eq!(
            receipt["mutationAvailable"].as_bool(),
            Some(expected),
            "{name}: receipt.mutationAvailable"
        );
    }
    if let Some(expected) = scenario.get("expectedOperation").and_then(Value::as_str) {
        assert_eq!(
            receipt["operation"].as_str(),
            Some(expected),
            "{name}: receipt.operation"
        );
    }

    match scenario.get("expectedFailureKind") {
        Some(Value::String(expected)) => assert_eq!(
            receipt["failureKind"].as_str(),
            Some(expected.as_str()),
            "{name}: receipt.failureKind"
        ),
        Some(Value::Null) => assert!(
            receipt["failureKind"].is_null(),
            "{name}: receipt.failureKind should be null"
        ),
        Some(_) => panic!("{name}: scenario.expectedFailureKind must be string or null"),
        None => {}
    }

    for expected in json_string_array(scenario, "verificationContains") {
        assert_receipt_verification_contains(name, receipt, expected);
    }
    for forbidden in json_string_array(scenario, "verificationExcludes") {
        assert_receipt_verification_excludes(name, receipt, forbidden);
    }
}

fn assert_compact_check(name: &str, scenario_dir: &Path, root: &Path, check: &Value) {
    let path = check["path"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: compactChecks[].path must be a string"));
    let query = check["query"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: compactChecks[].query must be a string"));

    let json_output = run_cli([
        "query".as_ref(),
        path.as_ref(),
        "--query".as_ref(),
        query.as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(json_output.status.success(), "{name}: {json_output:?}");
    let query_packet =
        serde_json::from_slice::<Value>(&json_output.stdout).expect("compact check query JSON");
    let match_value = select_query_match(
        name,
        "compactChecks[]",
        &query_packet,
        check.get("matchKind").and_then(Value::as_str),
        check.get("matchName").and_then(Value::as_str),
    );
    let projection = &match_value["projection"];
    let compact_code = compact_code_from_projection(projection)
        .unwrap_or_else(|| panic!("{name}: compact projection did not include rendered nodes"));
    assert_expected_compact_code(name, scenario_dir, check, &compact_code);
    for expected in json_string_array(check, "codeContains") {
        assert!(
            compact_code.contains(expected),
            "{name}: compact code missing {expected:?}:\n{compact_code}"
        );
    }
    for forbidden in json_string_array(check, "codeNotContains") {
        assert!(
            !compact_code.contains(forbidden),
            "{name}: compact code unexpectedly contained {forbidden:?}:\n{compact_code}"
        );
    }
    let selected_compact_code = match_value["code"].as_str().unwrap_or(&compact_code);
    assert_eq!(
        projection["compactSafety"]["literalPolicy"], "summarize",
        "{name}: compactSafety.literalPolicy"
    );
    assert_eq!(
        projection["compactSafety"]["whitespacePolicy"], "formatter-structural",
        "{name}: compactSafety.whitespacePolicy"
    );
    assert_eq!(
        projection["compactSafety"]["exactReadRequired"], true,
        "{name}: compactSafety.exactReadRequired"
    );
    assert_eq!(
        match_value["patchSafety"]["level"], "ast-patch-safe",
        "{name}: patchSafety.level"
    );
    assert_eq!(
        match_value["patchSafety"]["preimageSource"], "exact-read",
        "{name}: patchSafety.preimageSource"
    );
    assert!(
        match_value["patchSafety"]["allowedOperations"]
            .as_array()
            .is_some_and(|operations| operations
                .iter()
                .any(|operation| operation == "replace_item")),
        "{name}: patchSafety.allowedOperations"
    );

    let target = match_value["patchSafety"]["target"].clone();
    let exact_source = exact_read_from_target(root, &target);
    for expected in json_string_array(check, "exactContains") {
        assert!(
            exact_source.contains(expected),
            "{name}: exact formatted source missing {expected:?}:\n{exact_source}"
        );
    }
    if check
        .get("assertCompactShorter")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        assert!(
            selected_compact_code.len() < exact_source.len(),
            "{name}: compact code was not shorter than exact source\ncompact={}\nexact={}",
            selected_compact_code.len(),
            exact_source.len()
        );
    }
    if let Some(minimum) = check.get("minimumParserComplexity") {
        assert_minimum_parser_complexity(
            name,
            minimum,
            projection,
            selected_compact_code.len(),
            exact_source.len(),
        );
    }
    if let Some(minimum) = check.get("minimumFunctionalComplexity") {
        assert_minimum_functional_complexity(
            name,
            minimum,
            projection,
            selected_compact_code.len(),
            exact_source.len(),
        );
    }
    if check
        .get("rejectCompactAsPreimage")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        assert_compact_code_rejected_as_preimage(
            name,
            root,
            &target,
            &exact_source,
            selected_compact_code,
        );
    }
    if let Some(save_apply_patch) = check.get("saveApplyPatch").and_then(Value::as_str) {
        assert_saved_compact_apply_patch_passes(name, scenario_dir, save_apply_patch);
    }
}

fn assert_minimum_functional_complexity(
    name: &str,
    minimum: &Value,
    projection: &Value,
    compact_len: usize,
    exact_len: usize,
) {
    let responsibilities = projection["semanticResponsibilities"]
        .as_array()
        .unwrap_or_else(|| panic!("{name}: projection.semanticResponsibilities must be an array"));
    let responsibility_kinds = responsibilities
        .iter()
        .filter_map(|responsibility| responsibility["kind"].as_str())
        .collect::<BTreeSet<_>>();
    assert_min_usize(
        name,
        "minimumFunctionalComplexity.minDistinctResponsibilities",
        responsibility_kinds.len(),
        minimum
            .get("minDistinctResponsibilities")
            .and_then(Value::as_u64),
    );
    for required in json_string_array(minimum, "requiredResponsibilities") {
        assert!(
            responsibility_kinds.contains(required),
            "{name}: missing semantic responsibility {required:?}; got {responsibility_kinds:?}"
        );
    }
    for required in json_string_array(minimum, "requiredNativeParserResponsibilities") {
        assert!(
            responsibility_with_source(responsibilities, required, "native-parser"),
            "{name}: missing native-parser semantic responsibility {required:?}: {responsibilities:?}"
        );
    }
    for required in json_string_array(minimum, "requiredProjectionNodeResponsibilities") {
        assert!(
            responsibility_with_source(responsibilities, required, "projection-node"),
            "{name}: missing projection-node semantic responsibility {required:?}: {responsibilities:?}"
        );
    }
    if let Some(max_ratio) = minimum
        .get("maxCompactToExactRatio")
        .and_then(Value::as_f64)
    {
        let ratio = compact_len as f64 / exact_len.max(1) as f64;
        assert!(
            ratio <= max_ratio,
            "{name}: compact/exact ratio {ratio:.3} exceeds {max_ratio:.3}"
        );
    }
}

fn responsibility_with_source(responsibilities: &[Value], kind: &str, source: &str) -> bool {
    responsibilities.iter().any(|responsibility| {
        responsibility["kind"].as_str() == Some(kind)
            && responsibility["source"].as_str() == Some(source)
    })
}

fn assert_minimum_parser_complexity(
    name: &str,
    minimum: &Value,
    projection: &Value,
    compact_len: usize,
    exact_len: usize,
) {
    let nodes = projection["nodes"]
        .as_array()
        .unwrap_or_else(|| panic!("{name}: projection.nodes must be an array"));
    let max_depth = nodes
        .iter()
        .filter_map(|node| node["depth"].as_u64())
        .max()
        .unwrap_or(0);
    let roles = nodes
        .iter()
        .filter_map(|node| node["role"].as_str())
        .collect::<BTreeSet<_>>();
    let role_count = |role: &str| {
        nodes
            .iter()
            .filter(|node| node["role"].as_str() == Some(role))
            .count()
    };
    assert_min_usize(
        name,
        "minimumParserComplexity.minNodes",
        nodes.len(),
        minimum.get("minNodes").and_then(Value::as_u64),
    );
    assert_min_usize(
        name,
        "minimumParserComplexity.minDistinctRoles",
        roles.len(),
        minimum.get("minDistinctRoles").and_then(Value::as_u64),
    );
    assert_min_usize(
        name,
        "minimumParserComplexity.minControlFlowNodes",
        role_count("control-flow"),
        minimum.get("minControlFlowNodes").and_then(Value::as_u64),
    );
    assert_min_usize(
        name,
        "minimumParserComplexity.minTerminalNodes",
        role_count("terminal"),
        minimum.get("minTerminalNodes").and_then(Value::as_u64),
    );
    assert_min_usize(
        name,
        "minimumParserComplexity.minDepth",
        usize::try_from(max_depth).expect("projection depth fits usize"),
        minimum.get("minDepth").and_then(Value::as_u64),
    );
    if let Some(max_ratio) = minimum
        .get("maxCompactToExactRatio")
        .and_then(Value::as_f64)
    {
        let ratio = compact_len as f64 / exact_len.max(1) as f64;
        assert!(
            ratio <= max_ratio,
            "{name}: compact/exact ratio {ratio:.3} exceeds {max_ratio:.3}"
        );
    }
}

fn assert_min_usize(name: &str, label: &str, actual: usize, expected: Option<u64>) {
    if let Some(expected) = expected {
        assert!(
            actual >= usize::try_from(expected).expect("minimum fits usize"),
            "{name}: {label} expected at least {expected}, got {actual}"
        );
    }
}

fn assert_expected_compact_code(name: &str, scenario_dir: &Path, check: &Value, actual: &str) {
    if let Some(fixture) = check.get("codeFixture").and_then(Value::as_str) {
        let expected = read_fixture_text(scenario_dir, fixture);
        assert_eq!(
            actual,
            expected.trim_end(),
            "{name}: compact code fixture drift"
        );
    }
    if let Some(expected) = check.get("codeEquals").and_then(Value::as_str) {
        assert_eq!(actual, expected, "{name}: compact code mismatch");
    }
    if check
        .get("assertRustfmtStyleCompact")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let lines = actual.lines().collect::<Vec<_>>();
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("    ") && !line.trim().is_empty()),
            "{name}: compact code lost rustfmt-style nested indentation: {actual}"
        );
        assert!(
            lines.iter().any(|line| line.trim_end().ends_with('{')),
            "{name}: compact code lost visible block opening brace: {actual}"
        );
        assert!(
            lines.iter().any(|line| line.trim() == "}"),
            "{name}: compact code lost visible block closing brace: {actual}"
        );
    }
}

fn compact_code_from_projection(projection: &Value) -> Option<String> {
    let nodes = projection["nodes"].as_array()?;
    let state = nodes
        .iter()
        .filter_map(|node| {
            let depth = node["depth"].as_u64()? as usize;
            let label = node["label"].as_str()?.to_string();
            Some((depth, label))
        })
        .fold(
            CompactCodeRenderState::default(),
            |state, (depth, label)| state.push_node(depth, label),
        );
    let compact_code = state.finish();
    (!compact_code.trim().is_empty()).then_some(compact_code)
}

#[derive(Default)]
struct CompactCodeRenderState {
    lines: Vec<String>,
    open_depths: Vec<usize>,
}

impl CompactCodeRenderState {
    fn push_node(mut self, depth: usize, label: String) -> Self {
        let label = label.trim();
        let label_consumed = self.close_projection_blocks(depth, Some(label));
        if !label_consumed && !label.is_empty() {
            self.lines
                .push(format!("{}{}", "    ".repeat(depth), label));
        }
        if label.ends_with('{') {
            self.open_depths.push(depth);
        }
        self
    }

    fn finish(mut self) -> String {
        self.close_projection_blocks(0, None);
        self.lines.join("\n")
    }

    fn close_projection_blocks(&mut self, next_depth: usize, next_label: Option<&str>) -> bool {
        while self
            .open_depths
            .last()
            .is_some_and(|open_depth| *open_depth >= next_depth)
        {
            let open_depth = self.open_depths.pop().expect("checked open depth");
            let indent = "    ".repeat(open_depth);
            if open_depth == next_depth
                && let Some(label) = next_label
                && label.starts_with("else")
            {
                self.lines.push(format!("{indent}}} {label}"));
                return true;
            }
            self.lines.push(format!("{indent}}}"));
        }
        false
    }
}

fn assert_compact_code_rejected_as_preimage(
    name: &str,
    root: &Path,
    target: &Value,
    exact_source: &str,
    compact_code: &str,
) {
    let packet = json!({
        "target": target,
        "operation": {
            "op": "replace_item",
            "snippet": exact_source,
            "expectedSnippet": compact_code,
            "maxEdits": 1
        }
    })
    .to_string();
    let output = run_cli_with_stdin(
        [
            OsString::from("ast-patch"),
            OsString::from("dry-run"),
            OsString::from("--packet"),
            OsString::from("-"),
            root.as_os_str().to_os_string(),
        ],
        &packet,
    );
    assert!(output.status.success(), "{name}: {output:?}");
    let receipt = serde_json::from_slice::<Value>(&output.stdout).expect("receipt JSON");
    assert_eq!(
        receipt["status"], "failed",
        "{name}: compact code should not verify as exact preimage: {receipt}"
    );
    assert_eq!(
        receipt["failureKind"], "target-preimage-mismatch",
        "{name}: compact code should fail at preimage match: {receipt}"
    );
}

fn assert_saved_compact_apply_patch_passes(name: &str, scenario_dir: &Path, fixture: &str) {
    let save_patch = read_json(&scenario_dir.join(fixture));
    let path = save_patch["path"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: saved compact patch path must be a string"));
    let query = save_patch["query"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: saved compact patch query must be a string"));
    let op = save_patch["op"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: saved compact patch op must be a string"));
    assert_eq!(op, "replace_item", "{name}: saved compact patch op");

    let input = tempfile::tempdir()
        .unwrap_or_else(|error| panic!("{name}: failed to create tempdir: {error}"));
    copy_dir_recursive(&scenario_dir.join("input"), input.path());

    let query_packet = run_query_json(input.path(), path, query);
    let match_value = select_query_match(
        name,
        "saved compact input",
        &query_packet,
        save_patch.get("targetKind").and_then(Value::as_str),
        save_patch.get("targetName").and_then(Value::as_str),
    );
    let input_compact = compact_code_from_projection(&match_value["projection"])
        .unwrap_or_else(|| panic!("{name}: saved compact input projection did not render"));
    let input_fixture = save_patch["inputCompactFixture"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: inputCompactFixture must be a string"));
    assert_eq!(
        input_compact.trim_end(),
        read_fixture_text(scenario_dir, input_fixture).trim_end(),
        "{name}: input compact fixture should match"
    );

    assert_eq!(
        match_value["patchSafety"]["level"], "ast-patch-safe",
        "{name}: saved compact query target should be ast-patch-safe"
    );
    let target = match_value["patchSafety"]["target"].clone();
    let preimage = exact_read_from_target(input.path(), &target);
    if let Some(minimum) = save_patch.get("inputMinimumFunctionalComplexity") {
        assert_query_packet_functional_complexity(
            name,
            "saved compact input",
            minimum,
            input.path(),
            match_value,
            input_compact.trim_end(),
        );
    }

    let replacement_read = save_patch["replacementRead"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: replacementRead must be a string"));
    let replacement = exact_read_from_locator(&scenario_dir.join("expected"), replacement_read);
    let packet = json!({
        "target": target,
        "operation": {
            "op": "replace_item",
            "snippet": replacement,
            "expectedSnippet": preimage,
            "maxEdits": save_patch.get("maxEdits").and_then(Value::as_u64).unwrap_or(1)
        }
    })
    .to_string();
    let output = run_cli_with_stdin(
        [
            OsString::from("ast-patch"),
            OsString::from("apply"),
            OsString::from("--packet"),
            OsString::from("-"),
            input.path().as_os_str().to_os_string(),
        ],
        &packet,
    );
    assert!(
        output.status.success(),
        "{name}: saved apply patch failed: {output:?}"
    );
    assert!(
        output.stdout.is_empty(),
        "{name}: successful saved ast-patch apply should not print a receipt: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let expected = snapshot_dir(&scenario_dir.join("expected"));
    let actual = snapshot_dir(input.path());
    assert_eq!(
        actual, expected,
        "{name}: saved compact patch should match expected tree"
    );

    let expected_query_packet = run_query_json(input.path(), path, query);
    let expected_match_value = select_query_match(
        name,
        "saved compact expected",
        &expected_query_packet,
        save_patch.get("targetKind").and_then(Value::as_str),
        save_patch.get("targetName").and_then(Value::as_str),
    );
    let expected_compact = compact_code_from_projection(&expected_match_value["projection"])
        .unwrap_or_else(|| panic!("{name}: saved compact expected projection did not render"));
    let expected_fixture = save_patch["expectedCompactFixture"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: expectedCompactFixture must be a string"));
    assert_eq!(
        expected_compact.trim_end(),
        read_fixture_text(scenario_dir, expected_fixture).trim_end(),
        "{name}: expected compact fixture should match"
    );
    if let Some(minimum) = save_patch.get("expectedMinimumFunctionalComplexity") {
        assert_query_packet_functional_complexity(
            name,
            "saved compact expected",
            minimum,
            input.path(),
            expected_match_value,
            expected_compact.trim_end(),
        );
    }
}

fn assert_query_packet_functional_complexity(
    name: &str,
    label: &str,
    minimum: &Value,
    root: &Path,
    match_value: &Value,
    compact_code: &str,
) {
    let target = &match_value["patchSafety"]["target"];
    let exact_source = exact_read_from_target(root, target);
    let selected_compact_code = match_value["code"].as_str().unwrap_or(compact_code);
    let check_name = format!("{name}: {label}");
    assert_minimum_functional_complexity(
        &check_name,
        minimum,
        &match_value["projection"],
        selected_compact_code.len(),
        exact_source.len(),
    );
}

fn select_query_match<'a>(
    name: &str,
    label: &str,
    query_packet: &'a Value,
    kind: Option<&str>,
    item_name: Option<&str>,
) -> &'a Value {
    let matches = query_packet["matches"]
        .as_array()
        .unwrap_or_else(|| panic!("{name}: {label} query packet matches must be an array"));
    if kind.is_none() && item_name.is_none() {
        return matches
            .first()
            .unwrap_or_else(|| panic!("{name}: {label} query packet had no matches"));
    }
    matches
        .iter()
        .find(|match_value| {
            kind.is_none_or(|kind| match_value["kind"].as_str() == Some(kind))
                && item_name
                    .is_none_or(|item_name| match_value["name"].as_str() == Some(item_name))
        })
        .unwrap_or_else(|| {
            panic!(
                "{name}: {label} query packet did not contain match kind={kind:?} name={item_name:?}: {query_packet}"
            )
        })
}

fn run_query_json(root: &Path, path: &str, query: &str) -> Value {
    let output = run_cli([
        "query".as_ref(),
        path.as_ref(),
        "--query".as_ref(),
        query.as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice::<Value>(&output.stdout).expect("query packet JSON")
}

fn json_string_array<'a>(scenario: &'a Value, key: &str) -> Vec<&'a str> {
    scenario
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .unwrap_or_else(|| panic!("scenario.{key} entries must be strings"))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn json_array<'a>(scenario: &'a Value, key: &str) -> Vec<&'a Value> {
    scenario
        .get(key)
        .and_then(Value::as_array)
        .map(|values| values.iter().collect())
        .unwrap_or_default()
}

fn assert_receipt_verification_contains(name: &str, receipt: &Value, expected: &str) {
    let verification = receipt["verification"]
        .as_array()
        .unwrap_or_else(|| panic!("{name}: receipt.verification must be an array"));
    assert!(
        verification.iter().any(|value| value == expected),
        "{name}: missing verification {expected}: {verification:?}"
    );
}

fn assert_receipt_verification_excludes(name: &str, receipt: &Value, forbidden: &str) {
    let verification = receipt["verification"]
        .as_array()
        .unwrap_or_else(|| panic!("{name}: receipt.verification must be an array"));
    assert!(
        !verification.iter().any(|value| value == forbidden),
        "{name}: forbidden verification {forbidden}: {verification:?}"
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Entry {
    Dir,
    File(Vec<u8>),
}

fn snapshot_dir(root: &Path) -> BTreeMap<PathBuf, Entry> {
    let mut entries = BTreeMap::new();
    if root.is_dir() {
        snapshot_dir_recursive(root, root, &mut entries);
    }
    entries
}

fn snapshot_dir_recursive(base: &Path, dir: &Path, entries: &mut BTreeMap<PathBuf, Entry>) {
    let mut children = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("read dir {}: {error}", dir.display()))
        .map(|entry| entry.expect("dir entry").path())
        .collect::<Vec<_>>();
    children.sort();
    for path in children {
        if is_generated_fixture_artifact(&path) {
            continue;
        }
        let rel = path
            .strip_prefix(base)
            .unwrap_or_else(|error| panic!("strip prefix {}: {error}", path.display()))
            .to_path_buf();
        let metadata = fs::metadata(&path)
            .unwrap_or_else(|error| panic!("metadata {}: {error}", path.display()));
        if metadata.is_dir() {
            entries.insert(rel, Entry::Dir);
            snapshot_dir_recursive(base, &path, entries);
        } else if metadata.is_file() {
            let bytes = fs::read(&path)
                .unwrap_or_else(|error| panic!("read file {}: {error}", path.display()));
            entries.insert(rel, Entry::File(bytes));
        }
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
    let mut children = fs::read_dir(src)
        .unwrap_or_else(|error| panic!("read input dir {}: {error}", src.display()))
        .map(|entry| entry.expect("input dir entry").path())
        .collect::<Vec<_>>();
    children.sort();
    for path in children {
        if is_generated_fixture_artifact(&path) {
            continue;
        }
        let dest = dst.join(path.file_name().expect("input path file name"));
        let metadata = fs::metadata(&path)
            .unwrap_or_else(|error| panic!("metadata {}: {error}", path.display()));
        if metadata.is_dir() {
            fs::create_dir_all(&dest)
                .unwrap_or_else(|error| panic!("create dir {}: {error}", dest.display()));
            copy_dir_recursive(&path, &dest);
        } else if metadata.is_file() {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .unwrap_or_else(|error| panic!("create dir {}: {error}", parent.display()));
            }
            fs::copy(&path, &dest).unwrap_or_else(|error| {
                panic!("copy {} to {}: {error}", path.display(), dest.display())
            });
        }
    }
}

fn is_generated_fixture_artifact(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "target" || name == "Cargo.lock")
}

fn read_json(path: &Path) -> Value {
    let content =
        fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&content)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

fn read_fixture_text(scenario_dir: &Path, relative_path: &str) -> String {
    let path = scenario_dir.join(relative_path);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn exact_read_from_target(root: &Path, target: &Value) -> String {
    let read = target["read"]
        .as_str()
        .expect("patchSafety.target.read string");
    exact_read_from_locator(root, read)
}

fn exact_read_from_locator(root: &Path, read: &str) -> String {
    let (path, start, end) = parse_read_locator(read);
    let source_path = root.join(path);
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", source_path.display()));
    source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line_number = index + 1;
            (line_number >= start && line_number <= end).then_some(line)
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn parse_read_locator(read: &str) -> (&str, usize, usize) {
    let mut parts = read.rsplitn(3, ':');
    let end = parts
        .next()
        .and_then(|value| value.parse::<usize>().ok())
        .expect("read end line");
    let start = parts
        .next()
        .and_then(|value| value.parse::<usize>().ok())
        .expect("read start line");
    let path = parts.next().expect("read path");
    (path, start, end)
}

fn env_string(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).map(PathBuf::from)
}

fn env_flag(name: &str) -> bool {
    matches!(
        env::var(name).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

fn copy_target_file_to_temp(source_root: &Path, temp_root: &Path, relative_path: &str) {
    let source_path = source_root.join(relative_path);
    let dest_path = temp_root.join(relative_path);
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("create dir {}: {error}", parent.display()));
    }
    fs::copy(&source_path, &dest_path).unwrap_or_else(|error| {
        panic!(
            "copy real checkout source {} to {}: {error}",
            source_path.display(),
            dest_path.display()
        )
    });
}

fn scenario_name(dir: &Path) -> String {
    dir.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<scenario>")
        .to_string()
}
