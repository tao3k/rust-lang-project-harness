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
mounts the embedded cargo-test gate for the library target. That source-backed
gate covers unfiltered `cargo test --lib` and ordinary `cargo test` runs, which
keeps the harness rules honest before downstream projects inherit them.

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
rust_lang_project_harness::rust_project_harness_cargo_test_gate!(config = {
    rust_lang_project_harness::default_rust_harness_config()
});
```

Because the mount lives in the library test build, unfiltered `cargo test` and
`cargo test --lib` execute the project harness. The `#[cfg(test)]` guard keeps
normal `cargo build` free of the dev-dependency. Cargo test-name filters can
still skip any `#[test]` function, including this gate, so latency-sensitive
projects should add the build-time gate below when quick targeted checks must
not bypass project policy.

For filter-proof enforcement, add the harness as a build-dependency and call the
build gate from a thin `build.rs`:

```toml
[build-dependencies]
rust-lang-project-harness = { git = "https://github.com/tao3k/rust-lang-project-harness", branch = "main" }
```

```rust,ignore
fn main() {
    let config = rust_lang_project_harness::default_rust_harness_config();
    rust_lang_project_harness::assert_rust_project_harness_build_clean_from_env_with_config(
        &config,
    );
}
```

Build-script gates run before libtest applies name filters. They block
configured `Warning` and `Error` findings and emit Cargo `rerun-if-changed`
directives for conventional Rust project inputs, so source edits re-run the
gate even when a developer invokes a narrow filtered test.
When a harness-enabled package already has root `build.rs`, or already declares
the harness as a build-dependency, `RUST-PROJ-R012` reports an incomplete build
gate so the next Agent can finish the configuration during cargo test feedback.
A complete build gate is allowed to replace the source cargo-test gate.

Standalone Cargo test targets can also mount the direct gate when a project does
not have a source-backed cargo-test gate:

```rust
rust_lang_project_harness::rust_project_harness_gate!();
```

That covers a narrow test target directly. For library crates without a
build-time gate, prefer the source-backed cargo-test gate so one mount covers
both `cargo test` and `cargo test --lib`.

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
the package must expose either a source-tree
`rust_project_harness_cargo_test_gate!(config = ...)` mount or a complete
build-time harness gate so `cargo test --lib` cannot bypass project policy.
When either gate exists, root Cargo test targets can remain thin suite
aggregates without mounting another gate.

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
execute cargo bench, Criterion, Divan, iai-callgrind, k6, locust, chaos
injection, security scanners, or regression probes; it plans parser-native
obligations for external Agent skills and accepts receipts or waivers that clear
the compact reminder for the current task fingerprint. Each task carries
structured evidence requirements in JSON and an owner-grouped contract in
compact text:

```rust
use std::path::Path;

use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationProfileHint, default_rust_harness_config,
    plan_rust_project_verification_with_config, render_rust_verification_plan,
    render_rust_verification_skill_contracts,
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
let contract_tree = render_rust_verification_skill_contracts(&plan);
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

When an Agent does not yet know which owners need those profile hints, build a
parser-native profile index first. The index is a low-token configuration
draft: it inspects owner branches, public surfaces, local owner dependencies,
and Cargo dependency facts, then renders only missing or drifting profile
hints. Branch owners aggregate their child-module signals, while crate root
facades stay out of the way when they only re-export owner APIs. Third-party
dependency semantics stay project-owned: the harness maps
`use foo::...` through `Cargo.toml` dependency keys, `package = "..."` renames,
optional flags, features, and target/dev/build tables, but only a configured
`RustVerificationDependencySignal` turns that fact into persistence, network,
security, or performance responsibility. Once the project supplies a matching
`RustVerificationProfileHint`, the compact profile reminder disappears.
This Cargo layer is intentionally narrow: it gives Agents dependency facts for
owner-boundary configuration, while resolved graphs, platform evaluation, and
transitive supply-chain analysis remain future policy inputs rather than default
runtime cost.
Profile evidence separates `configured_dependency_roots` from
`unconfigured_dependency_roots` so the Agent can see whether a dependency
already has project-owned semantics or still needs a config decision.

```rust
use std::path::Path;

use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationDependencySignal,
    build_rust_verification_profile_index_with_config, default_rust_harness_config,
    render_rust_verification_profile_index,
};

let config = default_rust_harness_config().with_verification_dependency_signal(
    RustVerificationDependencySignal::new(
        "arrow-flight",
        [
            RustOwnerResponsibility::ExternalDependency,
            RustOwnerResponsibility::AvailabilityCritical,
        ],
    ),
);
let index = build_rust_verification_profile_index_with_config(Path::new("."), &config)?;
let compact_profile_advice = render_rust_verification_profile_index(&index);
let suggested_hints = index.active_profile_hints();
```

This is the intended Agent loop for diverse workspaces: inspect the profile
index, add or adjust owner-local hints through `RustHarnessConfig`, run the
verification planner, then satisfy tasks with receipts or waivers. The harness
does not force every crate into one testing shape; it gives the Agent compact
parser facts so crate-specific responsibilities can be configured deliberately.

```rust
use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationPhase, RustVerificationProfileHint,
    RustVerificationRequirement, RustVerificationSkillBinding, RustVerificationSkillDescriptor,
    RustVerificationTaskContract, RustVerificationTaskKind, default_rust_harness_config,
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
            )
            .with_rationale("this slice changes route authorization"),
    )
    .with_verification_responsibility_task_kinds(
        RustOwnerResponsibility::LatencySensitive,
        [RustVerificationTaskKind::Performance],
    )
    .with_verification_skill_binding(
        RustVerificationTaskKind::Performance,
        RustVerificationSkillBinding::new("rust-verification-performance")
            .with_adapter("criterion"),
    )
    .with_verification_skill_descriptor(
        RustVerificationSkillDescriptor::criterion_performance(),
    )
    .with_verification_skill_binding(
        RustVerificationTaskKind::Stress,
        RustVerificationSkillBinding::new("rust-verification-stress").with_adapter("k6"),
    )
    .with_verification_skill_descriptor(
        RustVerificationSkillDescriptor::k6_stress(),
    );
```

Global responsibility mappings and task contracts remain available for project
defaults. Owner-local `RustVerificationProfileHint::with_task_kinds()` and
`with_task_contract()` win only for that profile. Passing an empty task-kind set,
or using `without_verification_tasks()`, suppresses profile-derived reminders
for that owner while receipts and waivers still operate by task fingerprint.
When an owner-local task override changes the responsibility-derived default,
the profile must include a compact rationale. Without that rationale the harness
keeps a `responsibility_review` task active so the Agent explains why stress,
performance, security, chaos, or regression evidence was added or removed.

`RustVerificationSkillBinding` is the low-token bridge from Markdown skills to
code. When a task kind has a configured binding, compact output emits a short
dispatch hint such as `skill=rust-verification-performance@criterion` and omits
the repeated `requires`, `fact`, and `contract` onboarding lines. JSON still
keeps the full structured contract for tooling. When no binding exists, compact
text falls back to the passive progressive contract so an Agent can learn what
must be configured or executed. The binding label participates in the task
fingerprint, so changing adapters invalidates stale receipts.

`RustVerificationSkillDescriptor` is the next layer: a compact, typed execution
contract for a configured binding. It is not rendered by default. The default
verification line only adds `contract_ref=<skill>@<adapter>` when a descriptor
exists, and an Agent can call `render_rust_verification_skill_contracts(&plan)`
only when it needs to expand that reference into tool, command, pass/fail
standard, inputs, and receipt fields. Descriptor material also participates in
the task fingerprint, so changing the adapter contract invalidates stale
receipts without reintroducing long Markdown manuals into the hot prompt path.
Descriptor rendering is scoped to active tasks, so a passed receipt or complete
waiver also removes the on-demand contract expansion for that task.

The built-in descriptors keep stress and Rust-native performance separate.
`RustVerificationSkillDescriptor::k6_stress()` is for service-boundary pressure
and SLA evidence. `criterion_performance()`, `divan_performance()`, and
`iai_callgrind_performance()` are for Rust code-level benchmark, allocation,
instruction, cache, and profiling evidence through the `performance` task
family.

Performance receipts are also structured state, not prompt text. A Criterion,
Divan, or iai-callgrind run can attach searchable evidence such as
`benchmark_command`, `baseline`, `regression_threshold`,
`latency_or_throughput`, `allocation_profile`, and `profile_artifact` through
`RustVerificationReceipt::with_evidence(...)`. When that receipt matches the
current task fingerprint, compact output goes quiet, while
`render_rust_verification_plan_json()` still preserves `receipt_summary` and
`receipt_evidence` for dashboards, CI indexes, or later Agent retrieval.
Call `build_rust_verification_performance_index(&plan)` when a tool needs only
Rust performance state. The index is keyed by parser-owned package/owner facts,
fingerprint, state, skill binding, required evidence keys, receipt evidence,
artifact URI, and observed timestamp; it can be rendered as compact
`[perf-state]` text or JSON without expanding the long performance handbook.
It also exposes owner/package/state queries and missing receipt evidence keys,
so partial failed receipts can tell the Agent exactly which benchmark facts are
still absent.

Active verification tasks also carry `RustVerificationPlan::report_obligations`.
Compact verification output renders them as `[verify-report]` reminders so the
Agent knows to persist `verification_plan.json`, and `performance_index.json`
when performance tasks are active. Downstream workspaces decide where those
artifacts live, but the obligation to create a durable report is emitted by the
harness whenever the policy creates active verification work.
For integrations that want one fixed persistence contract,
`build_rust_verification_report_bundle(&plan)` and
`render_rust_verification_report_bundle_json(&plan)` materialize a small report
manifest. The manifest keeps artifacts modular and records template plus trace
guidance, including configurable runtime budgets for performance evidence.
Use `render_rust_verification_report_artifact_json(&plan, key)` to render one
artifact payload at a time instead of creating one large report JSON.
Artifacts are classified by persistence target: performance indexes default to
`source_baseline`, while verbose verification plans default to `runtime_cache`.

For workspaces, profile hint paths can be package-relative (`src/api.rs`) or
workspace-root-relative (`crates/api/src/api.rs`). Task fingerprints include the
owning package path, so two members with the same owner path do not collide.

## Current Rule Packs

Use `rust_rule_pack_descriptors()` for stable pack-level metadata. Default
project execution runs these packs in descriptor order:

- `rust.syntax`: blocks files that cannot be parsed by `syn`
- `rust.project_policy`: checks test layout, explicit test mounts, gate coverage, and thin root test targets
- `rust.modularity`: checks `lib.rs`/`mod.rs` facades, thin binary/build entrypoints, and source-shape drift
- `rust.agent_policy`: emits `AGENT-R001..R028` non-blocking advice for LLM repair

Rendered diagnostics are intentionally agent-first, not human audit reports.
When there are findings, compact text starts directly at the rule block: rule
id, `@ path:line:column` locator, `fix:`, source line when available, `Help:`,
and `Contract:`. It does not prepend global `Source`,
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

## Repo-Local Agent Skills

Top-level [`skills/`](skills/README.md) contains the Agent-facing operating
contracts for this repository. Use them as passive progressive guidance when
configuring or repairing harness policy, verification profiles, or
performance-sensitive Rust paths. Once `RustVerificationSkillBinding` is
configured, compact verification output should stay on the short code-level
dispatch path instead of making the Agent reread Markdown skill manuals.

## Docs

Detailed package material lives under [`docs/`](docs/index.md).
