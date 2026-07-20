pub(super) fn downstream_build_gate_agent_guidance(gate_label: &str) -> String {
    format!(
        "\
[rust-harness-agent-guidance]
gate: {gate_label}
trigger: cargo test runs the member build.rs before tests; keep rust-lang-project-harness under [build-dependencies].
repair:
- keep build.rs thin and call assert_rust_project_harness_downstream_policy_from_env.
- in a workspace, put common policy in the root harness/ module tree.
- construct RustProjectHarnessWorkspacePolicy once, then derive members with member_crate or member_crate_with_config.
- add crate-local owners, receipts, waivers, or report obligations in the member override only.
- rerun cargo test after updating policy or evidence.
"
    )
}
