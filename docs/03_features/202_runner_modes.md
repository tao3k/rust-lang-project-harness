# Runner Modes

The harness exposes two runner modes with different policy scope.

## Project Runner

Use `run_rust_project_harness()` or `assert_rust_project_harness_clean()` when a
caller has a project root. The project runner discovers conventional source and
test roots, builds a `RustProjectHarnessScope`, and runs all default rule packs.
With the default config, every Rust file under `src/`, `tests/`, `examples/`,
and `benches/` is in the harness, and root `build.rs` is included when it
exists, so this is the crate package-level gate:

1. `rust.syntax`
2. `rust.project_policy`
3. `rust.modularity`
4. `rust.agent_policy`

This is the mode used by `rust_project_harness_gate!`.

## Configuration

`RustHarnessConfig.source_dir_names` and `test_dir_names` are project-root
relative directories. Source-scoped rule packs use the resolved `source_paths` as
their ownership boundary, so custom source roots receive the same source-test,
modularity, and agent advice checks as `src`.

Package target paths such as root `build.rs`, `examples/`, and `benches/` are
tracked separately from source roots. They receive syntax checks and
package-scope path advice without becoming public source API for agent doc/name
advice.

`include_tests = true` is the default and keeps configured test roots inside the
package-level harness. `include_tests = false` is an explicit downgrade that
removes configured test roots from recursive parsing. It does not disable
filesystem-level project policy such as root test-layout and test-target gate
checks. Use the explicit-path runner for syntax-only probes.

## Explicit-Path Runner

Use `run_rust_lang_harness()` or `assert_rust_lang_harness_clean()` when a caller
only wants to inspect explicit files or directories. This runner has no project
scope, so project-scoped packs do not emit findings. The practical contract is:

1. `rust.syntax` still validates every discovered Rust file;
2. `rust.project_policy`, `rust.modularity`, and `rust.agent_policy` stay quiet
   because they require a project root and conventional ownership boundaries.

Use the project runner for repository policy gates. Use the explicit-path runner
for focused parser checks, editor integrations, and lightweight syntax probes.
