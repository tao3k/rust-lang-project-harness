use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use super::support::{normalize_temp_root, run_cli, write_clean_source, write_manifest};

#[test]
fn cli_renders_compact_text_by_default() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-compact");
    write_clean_source(root);

    let output = run_cli([root.as_os_str()]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert_eq!(stdout, "[ok] rust\n");
    assert!(!stdout.trim_start().starts_with('{'), "{stdout}");
}

#[test]
fn cli_json_flag_renders_structured_report() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-json");
    write_clean_source(root);

    let output = run_cli(["--json".as_ref(), root.as_os_str()]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("json report");
    assert_eq!(value["modules"].as_array().expect("modules").len(), 1);
    assert_eq!(value["findings"].as_array().expect("findings").len(), 0);
}

#[test]
fn cli_agent_snapshot_renders_reasoning_tree_summary() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-agent-snapshot");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain/mod.rs"),
        "//! Domain branch.\nmod leaf;\n",
    )
    .expect("write domain");
    fs::write(root.join("src/domain/leaf.rs"), "//! Domain leaf.\n").expect("write leaf");

    let output = run_cli(["--agent-snapshot".as_ref(), root.as_os_str()]);

    assert!(output.status.success(), "{output:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(stdout.starts_with("Modules:"), "{stdout}");
    assert!(stdout.contains("OwnerBranches:"), "{stdout}");
    assert!(!stdout.contains("FindingGroups:"), "{stdout}");
    assert!(!stdout.contains(" - none"), "{stdout}");
    assert!(!stdout.trim_start().starts_with('{'), "{stdout}");
    insta::assert_snapshot!("cli_agent_snapshot", stdout);
}

#[test]
fn cli_check_command_renders_policy_surface() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-check");
    write_clean_source(root);

    let compact = run_cli(["check".as_ref(), "--full".as_ref(), root.as_os_str()]);

    assert!(compact.status.success(), "{compact:?}");
    let stdout = String::from_utf8(compact.stdout).expect("utf8 stdout");
    assert_eq!(stdout, "[ok] rust\n");

    let json = run_cli([
        "check".as_ref(),
        "--changed".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);

    assert!(json.status.success(), "{json:?}");
    let stdout = String::from_utf8(json.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("json report");
    assert_eq!(value["modules"].as_array().expect("modules").len(), 1);
}

#[test]
fn cli_agent_provider_surface_delegates_hook_runtime_to_root_tool() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "codex-hooks");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Hook fixture.\n").expect("write source");

    let doctor = run_cli(["agent".as_ref(), "doctor".as_ref(), root.as_os_str()]);
    assert!(doctor.status.success(), "{doctor:?}");
    let stdout = String::from_utf8(doctor.stdout).expect("utf8 stdout");
    assert!(stdout.contains("runtime=semantic-agent-hook"), "{stdout}");

    let guide = run_cli(["guide".as_ref(), root.as_os_str()]);
    assert!(guide.status.success(), "{guide:?}");
    let stdout = String::from_utf8(guide.stdout).expect("guide stdout");
    assert!(
        stdout.starts_with("[agent-guide] lang=rust provider=asp-rust protocol=agent-guide.v1"),
        "{stdout}"
    );
    assert!(
        stdout.contains("|surface search purpose=tool-map output=search-guide code=false"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|catalog reasoningProfiles=owner-query,query-deps,owner-tests,finding-frontier,feature-cfg entries=owner-query,query-deps,owner-tests,finding-frontier,feature-cfg routes=path,read-frontier"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            r#"next="use search-guide command=search reasoning <profile> --owner/--query/--dependency ... --view seeds""#
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("<typed-selectors>"), "{stdout}");
    assert!(
        stdout.contains(r#"|refer search-guide="search guide ." use=low-frequency-tool-map"#),
        "{stdout}"
    );
    assert!(
        stdout.contains(r#"|refer query-guide="query guide ." use=code-stdout|read-plan-contract"#),
        "{stdout}"
    );
    assert!(
        stdout.contains(r#"|refer treesitter-query-guide="query guide treesitter .""#),
        "{stdout}"
    );
    assert!(
        !stdout.contains("|entry finding-frontier selectors=F:finding,O:owner?"),
        "{stdout}"
    );
    assert!(!stdout.contains("owner-items"), "{stdout}");
    assert!(!stdout.contains("profiles="), "{stdout}");

    let install = run_cli([
        "agent".as_ref(),
        "install".as_ref(),
        "--client".as_ref(),
        "codex".as_ref(),
        root.as_os_str(),
    ]);
    assert!(!install.status.success(), "{install:?}");
    assert!(
        String::from_utf8(install.stderr)
            .expect("stderr")
            .contains("rs-harness agent install moved to asp hook")
    );
    let legacy_guide = run_cli(["agent".as_ref(), "guide".as_ref(), root.as_os_str()]);
    assert!(!legacy_guide.status.success(), "{legacy_guide:?}");
    assert!(
        String::from_utf8(legacy_guide.stderr)
            .expect("stderr")
            .contains("rs-harness agent guide moved to rs-harness guide")
    );
    let hook = run_cli([
        "agent".as_ref(),
        "hook".as_ref(),
        "--client".as_ref(),
        "codex".as_ref(),
        "pre-tool".as_ref(),
        root.as_os_str(),
    ]);
    assert!(!hook.status.success(), "{hook:?}");
    assert!(
        String::from_utf8(hook.stderr)
            .expect("stderr")
            .contains("rs-harness agent hook moved to asp hook")
    );
    let guard = run_cli([
        "agent".as_ref(),
        "guard".as_ref(),
        "--client".as_ref(),
        "codex".as_ref(),
        root.as_os_str(),
        "--".as_ref(),
        "rtk".as_ref(),
        "read".as_ref(),
        "src/lib.rs".as_ref(),
    ]);
    assert!(!guard.status.success(), "{guard:?}");
    assert!(
        String::from_utf8(guard.stderr)
            .expect("stderr")
            .contains("rs-harness agent guard moved to asp hook")
    );

    let doctor = run_cli([
        "agent".as_ref(),
        "doctor".as_ref(),
        "--client".as_ref(),
        "codex".as_ref(),
        root.as_os_str(),
    ]);
    assert!(doctor.status.success(), "{doctor:?}");
    let stdout = String::from_utf8(doctor.stdout).expect("doctor stdout");
    assert!(
        stdout.starts_with("[agent-doctor] status=ok provider=rs-harness"),
        "{stdout}"
    );
}

#[test]
fn cli_keeps_agent_advice_non_blocking() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-advice");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod owned;\n").expect("write lib");
    fs::write(
        root.join("src/owned.rs"),
        "//! Owned module.\npub fn public_api() {}\n",
    )
    .expect("write owned module");

    let output = run_cli([root.as_os_str()]);

    assert!(output.status.success(), "{output:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(stdout.contains("AGENT-R002"), "{stdout}");
    assert!(!stdout.contains("[advice]"), "{stdout}");
    assert!(!stdout.contains("No blocking issues found."), "{stdout}");
    insta::assert_snapshot!("cli_agent_advice", stdout);
}

#[test]
fn cli_exits_nonzero_for_blocking_findings() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-blocking");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)] mod tests { #[test] fn it_works() {} }\n",
    )
    .expect("write lib");

    let output = run_cli([root.as_os_str()]);

    assert!(!output.status.success(), "{output:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(stdout.starts_with("[RUST-PROJ-R003]"), "{stdout}");
    assert!(stdout.contains("RUST-PROJ-R003"), "{stdout}");
    insta::assert_snapshot!("cli_blocking_finding", stdout);
}
