use rust_lang_project_harness::{
    RustProjectHarnessDependencyBaseline, RustProjectHarnessDownstreamPolicy,
    RustProjectHarnessWorkspacePolicy, assert_rust_project_harness_dependency_baseline,
    assert_rust_project_harness_downstream_policy,
    assert_rust_project_harness_verification_with_config, default_rust_harness_config,
    rust_downstream_verification_gate_guide_markdown,
};
use std::fs;
use tempfile::TempDir;

use crate::verification::support::write_api_project;

#[test]
fn build_gate_verification_requires_performance_and_stability_reports() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);

    let performance_only_config = default_rust_harness_config()
        .with_latency_sensitive_performance_owner(
            "src/api.rs",
            "API request path owns latency-sensitive dispatch",
        );
    let missing_stability = std::panic::catch_unwind(|| {
        assert_rust_project_harness_verification_with_config(
            root,
            &performance_only_config,
            "api crate",
        );
    })
    .expect_err("performance-only config should not satisfy full verification gate");
    let message = panic_message(missing_stability);
    assert!(
        message.contains("Stability verification tasks"),
        "{message}"
    );
    assert!(
        message.contains("[rust-harness-agent-guidance]"),
        "{message}"
    );
    assert!(
        message.contains("cargo test runs the member build.rs"),
        "{message}"
    );
    assert!(
        message.contains("RustProjectHarnessWorkspacePolicy"),
        "{message}"
    );
    assert!(message.contains("member_crate_with_config"), "{message}");

    let complete_config = default_rust_harness_config()
        .with_latency_sensitive_performance_owner(
            "src/api.rs",
            "API request path owns latency-sensitive dispatch",
        )
        .with_availability_stability_owner(
            "src/api.rs",
            "API request path must degrade and recover predictably",
        );
    assert_rust_project_harness_verification_with_config(root, &complete_config, "api crate");
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        return (*message).to_string();
    }
    "<non-string panic>".to_string()
}

#[test]
fn downstream_verification_gate_guide_classifies_api_and_cli_surfaces() {
    let guide = rust_downstream_verification_gate_guide_markdown();

    assert!(guide.contains("## Crate Layout"), "{guide}");
    assert!(guide.contains("## Classification"), "{guide}");
    assert!(guide.contains("Library/build.rs semantic gate"), "{guide}");
    assert!(
        guide.contains("CLI quick check and observation surface"),
        "{guide}"
    );
    assert!(guide.contains("harness/mod.rs"), "{guide}");
    assert!(guide.contains("owners.rs"), "{guide}");
    assert!(guide.contains("verification.rs"), "{guide}");
    assert!(
        guide.contains("RustProjectHarnessDownstreamPolicy"),
        "{guide}"
    );
    assert!(
        guide.contains("assert_rust_project_harness_downstream_policy_from_env"),
        "{guide}"
    );
    assert!(
        guide.contains("assert_rust_project_harness_verification_from_env_with_config"),
        "{guide}"
    );
    assert!(
        guide.contains("RustProjectHarnessDependencyBaseline"),
        "{guide}"
    );
    assert!(
        guide.contains("assert_rust_project_harness_dependency_baseline"),
        "{guide}"
    );
    assert!(guide.contains("dependencies.rs"), "{guide}");
    assert!(
        guide.contains("[rust-harness-dependency-guidance]"),
        "{guide}"
    );
    assert!(
        guide.contains("with_availability_stability_owner"),
        "{guide}"
    );
    assert!(
        guide.contains("Do not expose full verification as a standalone downstream CLI command"),
        "{guide}"
    );
    assert!(guide.contains("## Workspace Layout"), "{guide}");
    assert!(
        guide.contains("RustProjectHarnessWorkspacePolicy"),
        "{guide}"
    );
    assert!(guide.contains("member_crate_with_config"), "{guide}");
    assert!(
        guide.contains("A workspace should own common policy once"),
        "{guide}"
    );
    assert!(
        guide.contains("cargo test automatically triggers the member build.rs gate"),
        "{guide}"
    );
    assert!(guide.contains("[rust-harness-agent-guidance]"), "{guide}");
}

#[test]
fn downstream_policy_object_keeps_build_script_thin() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);

    let policy = RustProjectHarnessDownstreamPolicy::new(
        "api crate",
        default_rust_harness_config()
            .with_cargo_check_advice_allow_explanation(
                "downstream policy modules own advisory triage while build.rs stays thin",
            )
            .with_latency_sensitive_performance_owner(
                "src/api.rs",
                "API request path owns latency-sensitive dispatch",
            )
            .with_availability_stability_owner(
                "src/api.rs",
                "API request path must degrade and recover predictably",
            ),
    );

    let report = assert_rust_project_harness_downstream_policy(root, &policy);

    assert_eq!(policy.gate_label(), "api crate");
    assert!(report.is_clean(), "{report:?}");
}

#[test]
fn downstream_policy_asserts_dependency_baseline_from_cargo_lock() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    write_cargo_lock(
        root,
        r#"
version = 4

[[package]]
name = "rust-lang-project-harness"
version = "0.1.2"
source = "git+https://github.com/tao3k/agent-semantic-protocols?rev=abc123#abc123"
"#,
    );

    let policy = RustProjectHarnessDownstreamPolicy::new(
        "api crate",
        default_rust_harness_config()
            .with_cargo_check_advice_allow_explanation(
                "downstream policy modules own advisory triage while build.rs stays thin",
            )
            .with_latency_sensitive_performance_owner(
                "src/api.rs",
                "API request path owns latency-sensitive dispatch",
            )
            .with_availability_stability_owner(
                "src/api.rs",
                "API request path must degrade and recover predictably",
            ),
    )
    .with_dependency_baseline(
        RustProjectHarnessDependencyBaseline::new().require_git_package(
            "rust-lang-project-harness",
            "0.1.2",
            "rev=abc123",
        ),
    );

    let report = assert_rust_project_harness_downstream_policy(root, &policy);

    assert!(report.is_clean(), "{report:?}");
    assert_eq!(
        policy
            .dependency_baseline()
            .expect("dependency baseline")
            .packages()
            .len(),
        1
    );
}

#[test]
fn dependency_baseline_reports_duplicate_or_stale_entries_with_agent_guidance() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    write_cargo_lock(
        root,
        r#"
version = 4

[[package]]
name = "rust-lang-project-harness"
version = "0.1.1"
source = "git+https://github.com/tao3k/agent-semantic-protocols?rev=old#old"

[[package]]
name = "rust-lang-project-harness"
version = "0.1.2"
source = "git+https://github.com/tao3k/agent-semantic-protocols?rev=new#new"
"#,
    );
    let baseline = RustProjectHarnessDependencyBaseline::new().require_git_package(
        "rust-lang-project-harness",
        "0.1.2",
        "rev=new",
    );

    let duplicate = std::panic::catch_unwind(|| {
        assert_rust_project_harness_dependency_baseline(root, &baseline, "api crate");
    })
    .expect_err("duplicate lockfile package should fail baseline");
    let message = panic_message(duplicate);
    assert!(
        message.contains("requires exactly one Cargo.lock entry"),
        "{message}"
    );
    assert!(
        message.contains("[rust-harness-dependency-guidance]"),
        "{message}"
    );
    assert!(message.contains("cargo tree -i <package>"), "{message}");
    assert!(message.contains("rev=old"), "{message}");
    assert!(message.contains("rev=new"), "{message}");
}

#[test]
fn workspace_policy_reuses_common_config_for_member_crates() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);

    let workspace_policy = RustProjectHarnessWorkspacePolicy::new(
        "example-workspace",
        default_rust_harness_config()
            .with_cargo_check_advice_allow_explanation(
                "workspace policy owns common advisory triage",
            )
            .with_latency_sensitive_performance_owner(
                "src/api.rs",
                "workspace public API dispatch is latency-sensitive",
            )
            .with_availability_stability_owner(
                "src/api.rs",
                "workspace public API dispatch must degrade and recover predictably",
            ),
    );

    let api_policy = workspace_policy.member_crate("api");
    let report = assert_rust_project_harness_downstream_policy(root, &api_policy);

    assert_eq!(workspace_policy.workspace_label(), "example-workspace");
    assert_eq!(api_policy.gate_label(), "example-workspace::api");
    assert!(report.is_clean(), "{report:?}");

    let specialized_policy =
        workspace_policy.member_crate_with_config("api-specialized", |config| {
            config.with_cargo_check_advice_allow_explanation(
                "member policy owns a crate-local advisory exception",
            )
        });

    assert_eq!(
        specialized_policy.gate_label(),
        "example-workspace::api-specialized"
    );
    assert_eq!(
        workspace_policy
            .config()
            .cargo_check_advice_allow_explanation
            .as_deref(),
        Some("workspace policy owns common advisory triage")
    );
    assert_eq!(
        specialized_policy
            .config()
            .cargo_check_advice_allow_explanation
            .as_deref(),
        Some("member policy owns a crate-local advisory exception")
    );
}

#[test]
fn workspace_policy_shares_dependency_baseline_with_member_crates() {
    let workspace_policy =
        RustProjectHarnessWorkspacePolicy::new("example-workspace", default_rust_harness_config())
            .with_dependency_baseline(
                RustProjectHarnessDependencyBaseline::new().require_git_package(
                    "rust-lang-project-harness",
                    "0.1.2",
                    "rev=abc123",
                ),
            );

    let api_policy = workspace_policy.member_crate("api");

    assert_eq!(
        workspace_policy
            .dependency_baseline()
            .expect("workspace baseline")
            .packages()[0]
            .source_contains(),
        "rev=abc123"
    );
    assert_eq!(
        api_policy
            .dependency_baseline()
            .expect("member baseline")
            .packages()[0]
            .version(),
        "0.1.2"
    );
}

fn write_cargo_lock(root: &std::path::Path, contents: &str) {
    fs::write(root.join("Cargo.lock"), contents.trim_start()).expect("write Cargo.lock");
}
