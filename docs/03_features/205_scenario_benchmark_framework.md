# Scenario Benchmark Framework

Rust harness scenarios need a stable contract before they become performance
evidence. A scenario is not only an input fixture; it must also say what the
agent is meant to learn, which policy surface is under test, which benchmark
command measures it, and which speed and memory budgets protect the hot path.

The framework has three layers.

1. Contract validation is numeric and strict.
   The harness reads scenario metadata and benchmark receipts, then fails when
   required fields are missing or when observed speed or memory exceeds the
   configured budget.

2. Coverage validation is structural.
   The harness discovers every required scenario root and fails when a scenario
   is missing `benchmark.toml`. This prevents future scenarios from landing as
   unbounded fixtures.

3. Snapshot validation is semantic and stable.
   Tests render a normalized scenario receipt through `insta`. Dynamic values
   such as absolute paths, measured milliseconds, memory bytes, and timestamps
   are replaced with placeholders. The snapshot records the output shape,
   policy ids, agent-facing guidance, command hints, and pass/fail status.

This split keeps the performance gate useful. The numeric gate catches real
regressions; the snapshot catches confusing output or contract drift without
turning every machine-specific timing value into snapshot noise.

## Fixture Layout

Native Rust harness scenarios live under a bounded fixture root:

```text
tests/unit/scenarios/<group>/<scenario-id>/
  scenario.toml
  benchmark.toml
  inputs/
  expected/
  receipts/
```

`scenario.toml` describes the intent:

```toml
id = "control-flow-v1"
title = "Control-flow verification stays bounded"
policy_ids = ["RUST-CFG-R001"]
agent_goal = "Find the control-flow owner before editing."
inputs = "inputs"
expected = "expected"
```

`benchmark.toml` describes the performance contract:

```toml
bench_command = "cargo test -p rust-lang-project-harness control_flow_v1"
target_total_ms = 25
max_total_ms = 100
observed_total_ms = 18
regression_budget_ms = 20
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "Parser-native owner selection should remain a small fixture test."

[observed_timings]
parse_ms = 8
render_ms = 3
snapshot_ms = 2
```

Required fields are part of the contract. A scenario without speed, memory,
command, or rationale is invalid even if the fixture currently passes.

CLI AST patch scenarios use their existing manifest shape and are adapted into
the same benchmark contract:

```text
tests/fixtures/ast_patch_scenarios/<scenario-id>/
  scenario.json
  benchmark.toml
  input/
  expected/
  packet.json
```

Every directory with `scenario.toml` under `tests/unit/scenarios` and every
directory with `scenario.json` under `tests/fixtures/ast_patch_scenarios` is a
required benchmark scenario. The suite gate must fail when any of those roots is
missing `benchmark.toml`.

## Gate Semantics

The harness reports:

- `pass` when observed speed and memory stay within budget.
- `fail` when any observed value exceeds its budget.
- `invalid` when required metadata is missing or contradictory.

`observed_total_ms` must be less than or equal to `max_total_ms`.
`observed_memory_bytes` must be less than or equal to `memory_budget_bytes`.
`target_total_ms` and `regression_budget_ms` are explanatory fields that tell an
agent where optimization headroom remains.

The current default fixture budget for small AST patch scenario contracts is
`target_total_ms = 25`, `max_total_ms = 100`, and
`memory_budget_bytes = 8388608`. Wider gates need a scenario-specific rationale
in `target_rationale`; a large number without rationale is a contract failure in
review even when the numeric check passes.

## Insta Role

`insta` snapshots should include normalized receipts such as:

```text
scenario: control-flow-v1
status: pass
policies: RUST-CFG-R001
bench_command: cargo test -p rust-lang-project-harness control_flow_v1
observed_total_ms: <measured>
max_total_ms: 100
observed_memory_bytes: <measured>
memory_budget_bytes: 8388608
timings: parse_ms=<measured>, render_ms=<measured>, snapshot_ms=<measured>
agent_goal: Find the control-flow owner before editing.
```

The snapshot makes the agent-facing contract reviewable. The numeric assertions
remain in normal Rust tests so regressions are caught by values, not by snapshot
churn.
