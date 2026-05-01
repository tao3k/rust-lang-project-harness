# rust-lang-project-harness

`rust-lang-project-harness` is a project-level Rust language harness
library. It is a standalone extraction of the useful Rust project-governance
surface from `xiuxian-testing`, shaped like the Python project harness:
library-first APIs, deterministic rule catalogs, agent-facing compact rendered
diagnostics, and non-blocking `AGENT-*` advice for repair-oriented agents.

It also ships a small CLI for local and CI policy runs. Compact text is the
default output; pass `--json` when a structured `RustHarnessReport` payload is
needed, or `--agent-snapshot` when an LLM needs a low-noise reasoning-tree
summary instead of a full file list.

Project-root runners execute the full policy surface. By default they cover Rust
code under the crate's `src/`, `tests/`, `examples/`, and `benches/` roots, plus
root `build.rs` when it exists. If the root is a Cargo workspace or a directory
containing multiple Cargo packages, each package is evaluated with its own crate
scope. Explicit-path runners are focused syntax probes because they do not have
a project scope.

## Self-Apply Policy

This crate applies the default project harness to itself. `src/self_policy.rs`
mounts the embedded cargo-test gate for the library target, and
`tests/unit_test.rs` mounts the same default gate for the Cargo test target.
That keeps the harness rules honest: policy changes must pass through the
package's own rule packs before downstream projects inherit them.

Default assertions treat `Warning` and `Error` findings as blocking. `Info`
findings, including all `AGENT-*` advice, stay visible in compact rendered
diagnostics without failing the gate.

## Quick Use

For downstream projects, add the harness as a dev-dependency:

```toml
[dev-dependencies]
rust-lang-project-harness = { git = "https://github.com/tao3k/rust-lang-project-harness", branch = "main" }
```

Then mount the cargo-test gate from `src/lib.rs`:

```rust
#[cfg(test)]
rust_lang_project_harness::rust_project_harness_cargo_test_gate!();
```

Because the mount lives in the library test build, both `cargo test` and
`cargo test --lib` execute the project harness. The `#[cfg(test)]` guard keeps
normal `cargo build` free of the dev-dependency.

Root Cargo test targets can also mount the direct gate:

```rust
rust_lang_project_harness::rust_project_harness_gate!();
```

That covers `cargo test`, but it does not cover `cargo test --lib` unless the
library target also mounts the embedded cargo-test gate.

### Why `RUST-PROJ-R009` Exists

`RUST-PROJ-R009` is the policy that protects the `cargo test --lib` path. It is
intentionally narrower than Cargo's full resolver: the harness does not try to
evaluate every workspace, feature, target, or cfg combination a downstream
project may use. Instead, it looks for direct harness evidence.

A library crate is treated as harness-enabled when either its parsed
`Cargo.toml` dependency tables reference the canonical package
`rust-lang-project-harness`, or native Rust syntax contains an existing harness
gate macro. Comments, strings, and prose do not count.

The manifest parser checks ordinary dependency tables and target-specific
dependency tables, including Cargo dependency renames:

```toml
[dev-dependencies.local_harness]
package = "rust-lang-project-harness"
path = "../rust-lang-project-harness"
```

The dependency key can be local to the downstream project, but the package
identity remains `rust-lang-project-harness`. Once that direct evidence exists,
the library target must mount `rust_project_harness_cargo_test_gate!()` from the
source tree so `cargo test --lib` cannot bypass project policy.

The lower-level assertion API is available when a custom test shape is needed:

```rust
use std::path::Path;

use rust_lang_project_harness::assert_rust_project_harness_clean;

#[test]
fn rust_project_harness_gate() {
    assert_rust_project_harness_clean(Path::new(env!("CARGO_MANIFEST_DIR")));
}
```

For a compact repair surface without panicking:

```rust
use std::path::Path;

use rust_lang_project_harness::{
    render_rust_project_harness, run_rust_project_harness,
};

let report = run_rust_project_harness(Path::new(".")).expect("harness run");
println!("{}", render_rust_project_harness(&report));
```

The equivalent CLI keeps compact text as the default and exits nonzero only for
configured-blocking findings:

```shell
cargo run --bin rust-project-harness -- .
cargo run --bin rust-project-harness -- --json .
cargo run --bin rust-project-harness -- --agent-snapshot .
```

Library callers can tune policy without changing the default rule catalogs:

```rust
use std::path::Path;

use rust_lang_project_harness::{
    RustDiagnosticSeverity, RustRulePack, default_rust_harness_config,
    run_rust_project_harness_with_config,
};

let config = default_rust_harness_config()
    .with_rule_pack_severity(RustRulePack::Modularity, RustDiagnosticSeverity::Info)
    .with_rule_severity("RUST-MOD-R010", RustDiagnosticSeverity::Info)
    .with_disabled_rule("AGENT-R008");
let report =
    run_rust_project_harness_with_config(Path::new("."), &config).expect("harness run");
```

Rule ids can be disabled for a run or reclassified to another severity. The
configured `blocking_severities` still decide whether the final report fails.

Verification skills use a separate library-first contract. The harness does not
execute k6, locust, chaos injection, security scanners, or regression probes; it
plans parser-native obligations for external Agent skills and accepts receipts or
waivers that clear the compact reminder for the current task fingerprint. Each
task carries structured evidence requirements in JSON and an owner-grouped
contract in compact text:

```rust
use std::path::Path;

use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationProfileHint, default_rust_harness_config,
    plan_rust_project_verification_with_config, render_rust_verification_plan,
};

let config = default_rust_harness_config().with_verification_profile_hint(
    RustVerificationProfileHint::new(
        "src/api.rs",
        [RustOwnerResponsibility::PublicApi, RustOwnerResponsibility::LatencySensitive],
    ),
);
let plan =
    plan_rust_project_verification_with_config(Path::new("."), &config).expect("plan");
let compact = render_rust_verification_plan(&plan);
```

The compact verification renderer prints only active owner obligations such as
`[verify] src/api.rs` with task lines like
`|stress: pending phase=after_unit_tests_pass fingerprint=...`. A passed
`RustVerificationReceipt` or a complete `RustVerificationWaiver` tied to the
same fingerprint removes that task from compact output. Parser facts outrank
config hints, so incorrect responsibility declarations become
`responsibility_review` tasks instead of silently changing what the harness
believes.

The verification surface is configurable through the same library config. A
project can define global defaults, then let an Agent narrow one owner profile
when the task boundary is more specific than the global mapping. This lets the
Agent say "this owner is a public API, but this change needs security
evidence, not stress evidence", or "this owner has no external verification
task in this slice", without changing unrelated owners.

```rust
use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationPhase, RustVerificationProfileHint,
    RustVerificationRequirement, RustVerificationTaskContract, RustVerificationTaskKind,
    default_rust_harness_config,
};

let config = default_rust_harness_config()
    .with_verification_profile_hint(
        RustVerificationProfileHint::new("src/api.rs", [RustOwnerResponsibility::PublicApi])
            .with_task_kinds([RustVerificationTaskKind::Security])
            .with_task_contract(
                RustVerificationTaskKind::Security,
                RustVerificationTaskContract::new(
                    RustVerificationPhase::BeforeRelease,
                    "security skill must report tenant authz probes for this fingerprint",
                    [RustVerificationRequirement::new(
                        "tenant_authz",
                        "tenant authz probe result",
                    )],
                ),
            ),
    )
    .with_verification_responsibility_task_kinds(
        RustOwnerResponsibility::LatencySensitive,
        [RustVerificationTaskKind::Stress],
    );
```

Global responsibility mappings and task contracts remain available for project
defaults. Owner-local `RustVerificationProfileHint::with_task_kinds()` and
`with_task_contract()` win only for that profile. Passing an empty task-kind set,
or using `without_verification_tasks()`, suppresses profile-derived reminders
for that owner while receipts and waivers still operate by task fingerprint.

For workspaces, profile hint paths can be package-relative (`src/api.rs`) or
workspace-root-relative (`crates/api/src/api.rs`). Task fingerprints include the
owning package path, so two members with the same owner path do not collide.

## Current Rule Packs

Use `rust_rule_pack_descriptors()` for stable pack-level metadata. Default
project execution runs these packs in descriptor order:

- `rust.syntax`: blocks files that cannot be parsed by `syn`
- `rust.project_policy`: checks test layout, explicit test mounts, gate coverage, and thin root test targets
- `rust.modularity`: checks `lib.rs`/`mod.rs` facades, thin binary/build entrypoints, and source-shape drift
- `rust.agent_policy`: emits `AGENT-R001..R011` non-blocking advice for LLM repair

Rendered diagnostics are intentionally agent-first, not human audit reports.
When there are findings, compact text starts directly at the rule block: rule
id, source location, highlighted source line when available, one short source
pointer, `Help:`, and `Contract:`. It does not prepend global `Source`,
`Files`, `Parsed`, `Issues`, or `No blocking issues` headers. A fully clean run
uses only the minimal `[ok] rust` success signal. Structured audit consumers
should keep using the serializable `RustHarnessReport` shape through
`render_rust_project_harness_json()`.
Agents that need project shape rather than diagnostic cards can use
`render_rust_project_harness_agent_snapshot()` or the `--agent-snapshot` CLI
mode; that output starts with the module/owner facts agents need and omits
singleton package boilerplate, empty sections, and zero-valued drift counters.
Owner branches render parser-labeled child edges such as `mod:src/domain.rs`,
`path-mod:src/custom.rs`, and `include:src/shard.rs`, and package-level owner
dependencies render as compact edges such as
`src/lib.rs --crate--> src/domain.rs`.

## Docs

Detailed package material lives under [`docs/`](docs/index.md).
