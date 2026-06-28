use super::contract::RustScenarioBenchmarkContract;

pub(super) fn benchmark_entry_targets_contract_gate(
    benchmark: &RustScenarioBenchmarkContract,
) -> bool {
    [
        benchmark.test.as_deref(),
        benchmark.bench.as_deref(),
        benchmark.case.as_deref(),
        benchmark.snapshot.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| value.split_ascii_whitespace().any(targets_contract_gate))
}

fn targets_contract_gate(token: &str) -> bool {
    token == "assert_rule_fixture_scenario_benchmarks"
        || token.ends_with("_rule_fixtures_have_scenario_benchmarks")
}

pub(super) fn default_benchmark_toml_template() -> String {
    [
        "template:",
        "harness = \"libtest\"",
        "test = \"<focused-libtest-case>\"",
        "snapshot = \"<insta-snapshot-name>\"",
        "# For Criterion, Divan, or iai-callgrind use:",
        "# harness = \"criterion\"",
        "# bench = \"<cargo-bench-target>\"",
        "# case = \"<benchmark-group-or-function>\"",
        "target_total = \"25ms\"",
        "max_total = \"100ms\"",
        "observed_total = \"25ms\"",
        "regression_budget = \"20ms\"",
        "memory_budget_bytes = 8388608",
        "observed_memory_bytes = 4194304",
        "target_rationale = \"Small rule fixture should stay bounded.\"",
        "",
        "[input_expected_comparison]",
        "input_total = \"30ms\"",
        "expected_total = \"25ms\"",
        "input_memory_bytes = 6291456",
        "expected_memory_bytes = 4194304",
        "interpretation = \"Record both fixture timings; expected should normally be faster or more memory efficient than input.\"",
        "# expected_not_faster_annotation = \"Required when expected_total >= input_total or expected_memory_bytes >= input_memory_bytes.\"",
        "",
        "[observed_timings]",
        "contract_deserialize = \"8ms\"",
        "receipt_render = \"5ms\"",
        "snapshot_assertion = \"2ms\"",
    ]
    .join("\n")
}
