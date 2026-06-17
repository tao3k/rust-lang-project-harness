use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

use serde_json::Value;
use tempfile::TempDir;

use crate::cli::support::{run_cli, run_cli_with_env, run_search, write_manifest};

#[test]
fn cli_search_env_reports_toolchain_and_cfg_witnesses() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-search-env");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let toolchain = run_search(root, &["env", "toolchain"]);
    assert!(
        toolchain.starts_with("[search-env] q=toolchain pkg=."),
        "{toolchain}"
    );
    assert!(
        toolchain.contains("|env rustcVersion=")
            && toolchain.contains("source=rustc-version evidenceGrade=witness"),
        "{toolchain}"
    );
    assert!(
        toolchain.contains("|env cargoManifest edition=2024 resolver=- features=0 source=manifest manager=cargo evidenceGrade=fact"),
        "{toolchain}"
    );
    assert!(
        toolchain.contains(
            "|quality status=partial missing=cargo-metadata,resolved-features next=env:cfg"
        ),
        "{toolchain}"
    );

    let cfg = run_search(root, &["env", "cfg"]);
    assert!(cfg.starts_with("[search-env] q=cfg pkg=."), "{cfg}");
    assert!(
        cfg.contains("source=rustc-print-cfg evidenceGrade=witness"),
        "{cfg}"
    );
    assert!(
        cfg.contains(
            "|quality status=partial missing=cargo-metadata,resolved-feature-cfg next=cfg:<name>"
        ),
        "{cfg}"
    );
}

#[test]
fn cli_search_compare_env_stable_nightly_emits_compare_packet() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-search-compare-env");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let compare = run_search(root, &["compare", "env", "stable", "nightly"]);
    assert!(
        compare.starts_with("[search-compare] query=env_stable_nightly"),
        "{compare}"
    );
    assert!(
        compare.contains("authority=active-toolchain-vs-requested"),
        "{compare}"
    );
    assert!(
        compare.contains("|compare id=env-stable-nightly"),
        "{compare}"
    );
    assert!(
        compare.contains("|left channel=") && compare.contains("kind=active-toolchain"),
        "{compare}"
    );
    assert!(
        compare.contains("|right kind=requested-toolchain-set")
            && compare.contains("left=stable")
            && compare.contains("right=nightly"),
        "{compare}"
    );

    let compare_json = run_cli([
        "search".as_ref(),
        "compare".as_ref(),
        "env".as_ref(),
        "stable".as_ref(),
        "nightly".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(compare_json.status.success(), "{compare_json:?}");
    let value = serde_json::from_slice::<Value>(&compare_json.stdout).expect("compare json");
    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-compare-packet"
    );
    assert_eq!(value["languageId"], "rust");
    assert_eq!(value["namespace"], "compare");
    assert_eq!(value["authority"], "active-toolchain-vs-requested");
    assert_eq!(value["query"], "env stable nightly");
    assert_compare_packet_matches_schema(&value);
    assert_eq!(value["comparisons"][0]["id"], "env-stable-nightly");
    assert_eq!(value["comparisons"][0]["left"]["kind"], "active-toolchain");
    assert_eq!(
        value["comparisons"][0]["right"]["kind"],
        "requested-toolchain-set"
    );
    assert!(value["missing"].as_array().is_some(), "{value}");
}

#[test]
fn cli_search_compare_env_verified_with_fake_toolchain_evidence() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-search-compare-env-verified");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let value = compare_json_with_fake_toolchain(
        root,
        "stable-aarch64-apple-darwin (default)",
        &[
            "stable-aarch64-apple-darwin (default)",
            "nightly-aarch64-apple-darwin",
        ],
        "rustc 1.90.0 (verified-test)",
    );

    assert_compare_packet_matches_schema(&value);
    assert_eq!(value["quality"], "verified");
    assert_eq!(value["evidenceGrade"], "fact");
    assert_eq!(value["missing"], serde_json::json!([]));
    let comparison = &value["comparisons"][0];
    assert_eq!(comparison["result"], "active-toolchain-matches-left");
    assert_eq!(
        comparison["witness"],
        "rustup-active-toolchain-and-toolchain-list"
    );
    assert_eq!(comparison["left"]["channel"], "stable");
    assert_eq!(
        comparison["left"]["rustcVersion"],
        "rustc 1.90.0 (verified-test)"
    );
    assert_eq!(comparison["right"]["leftAvailable"], true);
    assert_eq!(comparison["right"]["rightAvailable"], true);
    assert!(quality_signals_include(
        comparison,
        "both-requested-toolchains-installed"
    ));
}

#[test]
fn cli_search_compare_env_reports_missing_requested_side_with_fake_toolchain_evidence() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-search-compare-env-missing-side");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let value = compare_json_with_fake_toolchain(
        root,
        "stable-aarch64-apple-darwin (default)",
        &["stable-aarch64-apple-darwin (default)"],
        "rustc 1.90.0 (insufficient-test)",
    );

    assert_compare_packet_matches_schema(&value);
    assert_eq!(value["quality"], "insufficient");
    assert_eq!(value["evidenceGrade"], "fact");
    assert_eq!(value["missing"], serde_json::json!(["toolchain:nightly"]));
    let comparison = &value["comparisons"][0];
    assert_eq!(comparison["result"], "requested-toolchain-unavailable");
    assert_eq!(
        comparison["witness"],
        "missing-rustup-or-requested-toolchain"
    );
    assert_eq!(comparison["right"]["leftAvailable"], true);
    assert_eq!(comparison["right"]["rightAvailable"], false);
    assert!(quality_signals_include(
        comparison,
        "comparison-side-missing"
    ));
}

#[test]
fn cli_search_code_comments_labels_claims_without_semantic_verdict() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-search-code-comments");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Module promise.\n// Runtime claim.\npub fn value() -> usize { 1 }\n",
    )
    .expect("write lib");

    let output = run_search(root, &["code", "comments", "--owner", "src/lib.rs"]);

    assert!(
        output.starts_with("[search-code] q=comments pkg=. claim=2 fact=0 witness=0"),
        "{output}"
    );
    assert!(
        output.contains("|claim kind=module-doc-comment owner=src/lib.rs line=1 evidenceGrade=claim evidence=comment verdict=unverified text=Module_promise."),
        "{output}"
    );
    assert!(
        output.contains("|claim kind=line-comment owner=src/lib.rs line=2 evidenceGrade=claim evidence=comment verdict=unverified text=Runtime_claim."),
        "{output}"
    );
    assert!(
        output.contains(
            "|quality status=partial missing=parser-verdict,witness next=owner:src/lib.rs"
        ),
        "{output}"
    );
}

fn compare_json_with_fake_toolchain(
    root: &Path,
    active_toolchain: &str,
    installed_toolchains: &[&str],
    rustc_version: &str,
) -> Value {
    let fake_bin = root.join("fakebin");
    write_fake_toolchain(
        &fake_bin,
        active_toolchain,
        installed_toolchains,
        rustc_version,
    );
    let output = run_cli_with_env(
        [
            "search".as_ref(),
            "compare".as_ref(),
            "env".as_ref(),
            "stable".as_ref(),
            "nightly".as_ref(),
            "--json".as_ref(),
            root.as_os_str(),
        ],
        [("PATH", fake_path(&fake_bin).as_os_str())],
    );
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice::<Value>(&output.stdout).expect("compare json")
}

fn write_fake_toolchain(
    bin_dir: &Path,
    active_toolchain: &str,
    installed_toolchains: &[&str],
    rustc_version: &str,
) {
    fs::create_dir_all(bin_dir).expect("create fake toolchain bin");
    let installed_lines = installed_toolchains
        .iter()
        .map(|toolchain| format!("printf '%s\\n' {}\n", shell_single_quote(toolchain)))
        .collect::<String>();
    let rustup = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = \"show\" ] && [ \"$2\" = \"active-toolchain\" ]; then\n\
         printf '%s\\n' {}\n\
         exit 0\n\
         fi\n\
         if [ \"$1\" = \"toolchain\" ] && [ \"$2\" = \"list\" ]; then\n\
         {}\
         exit 0\n\
         fi\n\
         exit 1\n",
        shell_single_quote(active_toolchain),
        installed_lines
    );
    let rustc = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = \"-Vv\" ]; then\n\
         printf '%s\\n' {}\n\
         exit 0\n\
         fi\n\
         exit 1\n",
        shell_single_quote(rustc_version)
    );
    write_executable(&bin_dir.join("rustup"), &rustup);
    write_executable(&bin_dir.join("rustc"), &rustc);
    #[cfg(windows)]
    write_windows_toolchain_commands(
        bin_dir,
        active_toolchain,
        installed_toolchains,
        rustc_version,
    );
}

fn write_executable(path: &Path, text: &str) {
    fs::write(path, text).expect("write executable");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("chmod executable");
    }
}

#[cfg(windows)]
fn write_windows_toolchain_commands(
    bin_dir: &Path,
    active_toolchain: &str,
    installed_toolchains: &[&str],
    rustc_version: &str,
) {
    let installed_lines = installed_toolchains
        .iter()
        .map(|toolchain| format!("echo {}\r\n", cmd_escape(toolchain)))
        .collect::<String>();
    fs::write(
        bin_dir.join("rustup.cmd"),
        format!(
            "@echo off\r\n\
             if \"%1\"==\"show\" if \"%2\"==\"active-toolchain\" (\r\n\
             echo {}\r\n\
             exit /b 0\r\n\
             )\r\n\
             if \"%1\"==\"toolchain\" if \"%2\"==\"list\" (\r\n\
             {}\
             exit /b 0\r\n\
             )\r\n\
             exit /b 1\r\n",
            cmd_escape(active_toolchain),
            installed_lines
        ),
    )
    .expect("write rustup command shim");
    fs::write(
        bin_dir.join("rustc.cmd"),
        format!(
            "@echo off\r\n\
             if \"%1\"==\"-Vv\" (\r\n\
             echo {}\r\n\
             exit /b 0\r\n\
             )\r\n\
             exit /b 1\r\n",
            cmd_escape(rustc_version)
        ),
    )
    .expect("write rustc command shim");
}

#[cfg(windows)]
fn cmd_escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '^' | '&' | '|' | '<' | '>' | '(' | ')' => ['^', character],
            _ => ['\0', character],
        })
        .filter(|character| *character != '\0')
        .collect()
}

fn fake_path(bin_dir: &Path) -> OsString {
    let mut paths = vec![bin_dir.to_path_buf()];
    if let Some(path) = env::var_os("PATH") {
        paths.extend(env::split_paths(&path));
    }
    env::join_paths(paths).expect("join PATH")
}

fn shell_single_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

fn assert_compare_packet_matches_schema(packet: &Value) {
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("schemas")
        .join("semantic-compare-packet.v1.schema.json");
    let schema = serde_json::from_slice::<Value>(&fs::read(&schema_path).expect("read schema"))
        .expect("parse schema");
    let packet_object = packet.as_object().expect("packet object");
    assert_required_fields(packet_object, &schema["required"]);
    assert_allowed_keys(packet_object.keys(), &schema["properties"]);
    assert_const_fields(packet, &schema["properties"]);
    assert_enum_contains(
        &schema["properties"]["evidenceGrade"],
        &packet["evidenceGrade"],
    );
    assert_enum_contains(&schema["properties"]["quality"], &packet["quality"]);

    let comparison_schema = &schema["$defs"]["comparison"];
    for comparison in packet["comparisons"].as_array().expect("comparisons") {
        let comparison_object = comparison.as_object().expect("comparison object");
        assert_required_fields(comparison_object, &comparison_schema["required"]);
        assert_allowed_keys(comparison_object.keys(), &comparison_schema["properties"]);
        assert_enum_contains(
            &comparison_schema["properties"]["evidenceGrade"],
            &comparison["evidenceGrade"],
        );
    }
}

fn assert_required_fields(object: &serde_json::Map<String, Value>, required: &Value) {
    for field in required.as_array().expect("required array") {
        let field = field.as_str().expect("required string");
        assert!(object.contains_key(field), "missing required field {field}");
    }
}

fn assert_allowed_keys<'a>(keys: impl Iterator<Item = &'a String>, properties: &Value) {
    let properties = properties.as_object().expect("properties object");
    for key in keys {
        assert!(properties.contains_key(key), "schema-unknown field {key}");
    }
}

fn assert_const_fields(packet: &Value, properties: &Value) {
    for (field, property) in properties.as_object().expect("properties object") {
        if let Some(expected) = property.get("const") {
            assert_eq!(&packet[field], expected, "const field {field}");
        }
    }
}

fn assert_enum_contains(schema_property: &Value, actual: &Value) {
    let variants = schema_property["enum"].as_array().expect("enum array");
    assert!(
        variants.iter().any(|variant| variant == actual),
        "{actual} not in {variants:?}"
    );
}

fn quality_signals_include(comparison: &Value, expected: &str) -> bool {
    comparison["qualitySignals"]
        .as_array()
        .is_some_and(|signals| {
            signals
                .iter()
                .any(|signal| signal.as_str() == Some(expected))
        })
}

#[test]
fn cli_search_extension_tokio_uses_manifest_and_source_derived_boundary_facts() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"cli-search-extension-tokio\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [dependencies]\n\
         tokio = { version = \"1\", features = [\"rt\", \"time\", \"process\"] }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "use std::time::Duration;\nuse tokio::time::timeout;\n\npub async fn bounded() {\n    let _ = timeout(Duration::from_secs(1), async {}).await;\n}\n",
    )
    .expect("write lib");

    let output = run_search(root, &["extension", "tokio"]);

    assert!(
        output.starts_with("[search-extension] q=tokio pkg=. extension=tokio dep=1 own=1"),
        "{output}"
    );
    assert!(
        output.contains("|extension tokio status=activated source=manifest evidenceGrade=fact"),
        "{output}"
    );
    assert!(
        output.contains("|owner src/lib.rs hit_kind=extension-usage extension=tokio"),
        "{output}"
    );
    assert!(
        output.contains("|extension-guidance dep=tokio usageLevel=capability_boundary engineeringBoundary=present ownerUsage=1"),
        "{output}"
    );
    assert!(
        output.contains("source=provider-capability-catalog evidenceGrade=fact"),
        "{output}"
    );
}
