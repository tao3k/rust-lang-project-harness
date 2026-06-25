pub(super) fn bench_command_targets_contract_gate(command: &str) -> bool {
    command.split_ascii_whitespace().any(|token| {
        token == "assert_rule_fixture_scenario_benchmarks"
            || token.ends_with("_rule_fixtures_have_scenario_benchmarks")
    })
}

pub(super) fn default_benchmark_toml_template() -> String {
    [
        "template:",
        "bench_command = \"cargo test <focused-test>\"",
        "target_total_ms = 25",
        "max_total_ms = 100",
        "observed_total_ms = 25",
        "regression_budget_ms = 20",
        "memory_budget_bytes = 8388608",
        "observed_memory_bytes = 4194304",
        "target_rationale = \"Small rule fixture should stay bounded.\"",
        "",
        "[observed_timings]",
        "fixture_ms = 25",
    ]
    .join("\n")
}
