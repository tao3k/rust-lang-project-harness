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

This is the mode used by `rust_project_harness_gate!` and
`rust_project_harness_cargo_test_gate!`.

When the requested root is a Cargo workspace or a directory that contains
multiple nested `Cargo.toml` package manifests, the project runner evaluates
each package as its own member scope. Test layout, `lib.rs` facade policy,
source-backed test mounts, and module reachability are therefore checked against
the owning crate root instead of the workspace directory. Workspace package
facts come from the shared Cargo manifest parser, so discovery and policy use
the same `Cargo.toml` interpretation.

## Cargo Test Embedding

Downstream crates can load the harness as a dev-dependency and mount it from the
library target:

```rust
#[cfg(test)]
rust_lang_project_harness::rust_project_harness_cargo_test_gate!(config = {
    rust_lang_project_harness::default_rust_harness_config()
        .with_verification_profile_hint(
            rust_lang_project_harness::RustVerificationProfileHint::new(
                "src/lib.rs",
                [rust_lang_project_harness::RustOwnerResponsibility::PublicApi],
            ),
        )
});
```

Place that line in `src/lib.rs`, or in a source module that `src/lib.rs`
declares. The `#[cfg(test)]` guard is part of the contract because
dev-dependencies are not available to normal `cargo build`, while `cargo test`
and `cargo test --lib` both compile the library test target.
The `config = { ... }` block is part of the Agent contract: it prevents a source
gate from silently accepting the default empty verification surface.

Root Cargo test targets under `tests/*.rs` can stay as thin suite aggregates
once the library target mounts a configured
`rust_project_harness_cargo_test_gate!(config = ...)`. That source-embedded
gate covers ordinary `cargo test` and closes the `cargo test --lib` escape path.

The cargo-test gate treats non-blocking `rust.agent_policy` advice as test
feedback by default. The core project runner still keeps `Info` findings
non-blocking, but `cargo test` normally hides passing test output, so the
embedded gate fails when compact agent advice exists and lets the next Agent see
the exact repair contract. The notification disappears when the agent fixes the
structure, or when the crate config explicitly suppresses or replaces the
applicable rule surface.

Use `advice = allow, config = { ... }` only for a deliberate legacy waiver where
cargo tests must pass even while rendered harness reports still expose advisory
findings:

```rust
#[cfg(test)]
rust_lang_project_harness::rust_project_harness_cargo_test_gate!(
    advice = allow,
    config = {
        rust_lang_project_harness::default_rust_harness_config()
            .with_verification_profile_hint(
                rust_lang_project_harness::RustVerificationProfileHint::new(
                    "src/lib.rs",
                    [rust_lang_project_harness::RustOwnerResponsibility::PublicApi],
                ),
            )
    }
);
```

Harness-enabled library projects are checked by `RUST-PROJ-R009`: once the
project has the harness dependency or another harness gate, a `src/lib.rs`
target must also expose a cargo-test gate from the source tree.

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

Policy findings are configurable through `RustHarnessConfig` after rule
evaluation and before the report is returned. `disabled_rules` removes matching
rule ids from the final finding list, while `rule_severity_overrides` changes a
matching finding's severity for that run. The `with_disabled_rule`,
`with_disabled_rules`, `with_disabled_rule_pack`, `with_rule_severity`,
`with_rule_pack_severity`, and `with_blocking_severities` builder methods
provide the stable library API for those controls. Pack-level helpers use the
`RustRulePack` enum and expand into the same rule-id collections, so the
serialized config shape remains unchanged. This keeps the default catalogs
deterministic while giving downstream crates a narrow way to turn a rule or pack
into advisory output or suppress rules they have intentionally replaced with
local policy.

## Explicit-Path Runner

Use `run_rust_lang_harness()` or `assert_rust_lang_harness_clean()` when a caller
only wants to inspect explicit files or directories. This runner has no project
scope, so project-scoped packs do not emit findings. The practical contract is:

1. `rust.syntax` still validates every discovered Rust file;
2. `rust.project_policy`, `rust.modularity`, and `rust.agent_policy` stay quiet
   because they require a project root and conventional ownership boundaries.

Use the project runner for repository policy gates. Use the explicit-path runner
for focused parser checks, editor integrations, and lightweight syntax probes.
