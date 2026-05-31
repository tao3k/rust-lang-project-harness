# rust-lang-project-harness

`rust-lang-project-harness` is a project-level Rust language harness for
repair-oriented coding agents. Its job is not to replace `rustc`, `rustfmt`, or
Clippy. Its job is to parse a Rust project through native Rust syntax and Cargo
manifest facts, turn that project into package/module/owner/dependency facts,
and expose compact policy feedback that lets an Agent keep writing Rust without
searching a massive file list every time.

The harness exists because LLM-written Rust tends to drift in predictable ways:
scope gets broad, files grow, `lib.rs` and `build.rs` start owning
implementation, `use super::*` hides dependencies, public APIs become primitive
or stringly typed, and verification duties are forgotten after the first green
test. The harness makes those structural risks explicit. It builds a
parser-native reasoning tree, evaluates deterministic rule packs, and returns
small repair contracts that an Agent can act on during `cargo check` or CI.

Humans can read the output, but they are not the primary interface. Compact text
is designed for Agents: stable rule id, `@ path:line:column`, `fix:`, optional
`line:`, `Help:`, and `Contract:`. It avoids human audit headers, code-frame
ornament, package counters, empty sections, and long JSON by default. Structured
tooling can still use `render_rust_project_harness_json()`, while Agents that
need project shape can use `--agent-snapshot` for a low-noise reasoning-tree
summary instead of a raw file inventory.

It also ships a small CLI for local and CI policy runs. Compact text is the
default output; pass `--json` when a structured `RustHarnessReport` payload is
needed, or `--agent-snapshot` when an LLM needs the project reasoning tree.

Project-root runners execute the full policy surface. By default they cover Rust
code under the crate's `src/`, `tests/`, `examples/`, and `benches/` roots, plus
root `build.rs` when it exists. If the root is a Cargo workspace or a directory
containing multiple Cargo packages, each package is evaluated with its own crate
scope. Explicit-path runners are focused syntax probes because they do not have
a project scope.

The expected loop is library-first and Agent-complete:

1. a downstream Rust crate adds the harness build-dependency and mounts the
   build-time gate;
2. `cargo check` runs the build script with the crate's parser-native project
   facts before the test/evaluation layer;
3. missing configuration, structural drift, or verification obligations render
   as compact findings;
4. the next Agent edits code or `RustHarnessConfig` until the finding naturally
   disappears, or records an explicit project-local rationale.

That loop is the point of the crate. The harness should make the correct next
Agent action visible without requiring a human to read a long handbook, inspect
every file, or infer project ownership from search results.

The Cargo layer boundary is explicit. `cargo check` is the primary harness gate
for parser-native policy: syntax, Cargo manifest facts, module and owner graph,
import clarity, scope coverage, build-gate closure, and verification planning
obligations. `cargo test` is only for test-layer semantics: legacy source gate
compatibility, explicit advice allowance, and future policies that consume real
runtime test or verification receipts.

## Self-Apply Policy

This crate applies the default project harness to itself. `src/self_policy.rs`
mounts the embedded cargo-test gate for the library target because the harness
crate cannot build-depend on itself. Downstream crates should use the
build-time gate below so `cargo check` runs parser-native policy before
`cargo test` exists in the loop.

Library runner assertions treat `Warning` and `Error` findings as blocking.
`Info` findings, including all `AGENT-*` advice, stay visible in compact
rendered diagnostics without failing plain library assertions. Cargo-check and
legacy cargo-test gates additionally promote agent advice into repair feedback
unless the config carries an explicit layer-specific explanation.

## Quick Use

For downstream projects, add the harness as a build-dependency:

```toml
[build-dependencies]
rust-lang-project-harness = { git = "https://github.com/tao3k/rust-lang-project-harness", branch = "main" }
```

Then call the build gate from a thin root `build.rs`:

```rust,ignore
fn main() {
    let config = rust_lang_project_harness::default_rust_harness_config();
    rust_lang_project_harness::assert_rust_project_harness_cargo_check_clean_from_env_with_config(
        &config,
    );
}
```

Build-script gates run during `cargo check`, before libtest, test filters, or
runtime evaluation. They block configured `Warning` and `Error` findings, treat
`Info` agent advice as repair feedback by default, and emit Cargo
`rerun-if-changed` directives for conventional Rust project inputs. A crate can
allow advisory findings only by adding an explicit
`with_cargo_check_advice_allow_explanation(...)` rationale to its config.

When a harness-enabled package lacks that build-time closure,
`RUST-PROJ-R012` reports the incomplete `cargo check` gate so the next Agent can
finish the configuration.

Cargo-test gates are still supported as a compatibility mount, but they are no
longer the preferred downstream entrypoint:

```rust
#[cfg(test)]
rust_lang_project_harness::rust_project_harness_gate!();
```

Use them only when a crate cannot yet add a build script. The policy reminder
will still prefer the `build.rs` gate because current harness policy is
parser-native and does not need the test/evaluation layer. In harness-enabled
packages, legacy cargo-test mounts emit `RUST-PROJ-R006` or `RUST-PROJ-R009`
warnings so the next Agent gets an explicit migration path instead of treating
the old gate as the final design.

### Why `RUST-PROJ-R012` Exists

`RUST-PROJ-R012` is the policy that protects the primary `cargo check` path. It
is intentionally narrower than Cargo's full resolver: the harness does not try
to evaluate every workspace, feature, target, or cfg combination a downstream
project may use. Instead, it looks for direct harness evidence.

A library crate is treated as harness-enabled when either its parsed
`Cargo.toml` dependency tables reference the canonical package
`rust-lang-project-harness`, or a native Rust build script already calls the
build gate. Comments, strings, and prose do not count.

The manifest parser checks ordinary dependency tables and target-specific
dependency tables, including Cargo dependency renames:

```toml
[dev-dependencies.local_harness]
package = "rust-lang-project-harness"
path = "../rust-lang-project-harness"
```

The dependency key can be local to the downstream project, but the package
identity remains `rust-lang-project-harness`. Once that direct evidence exists,
the package must expose a complete build-time harness gate so `cargo check`
cannot bypass parser-native project policy. Root Cargo test targets can remain
thin suite aggregates without mounting another gate.

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
cargo run --features cli --bin rs-harness -- .
cargo run --features cli --bin rs-harness -- --json .
cargo run --features cli --bin rs-harness -- --agent-snapshot .
cargo run --features cli --bin rs-harness -- search prime .
cargo run --features cli --bin rs-harness -- search prime --json .
cargo run --features cli --bin rs-harness -- search owner src/lib.rs items .
cargo run --features cli --bin rs-harness -- search dependency serde items public-api docs tests .
cargo run --features cli --bin rs-harness -- search api render_rust_project_harness .
cargo run --features cli --bin rs-harness -- search public-external-types .
cargo run --features cli --bin rs-harness -- search deps serde .
cargo run --features cli --bin rs-harness -- search deps serde/de@1::DeserializeOwned .
cargo run --features cli --bin rs-harness -- search cfg tokio_unstable .
cargo run --features cli --bin rs-harness -- check --changed .
cargo run --features cli --bin rs-harness -- agent doctor .
cargo run --features cli --bin rs-harness -- agent doctor --json .
```

The library exposes the search renderers behind the default `search` feature.
The binary is intentionally feature-gated behind `cli`, so downstream library
users do not build a CLI unless they opt in. Search output is compact RFC line
protocol by default and is meant to replace broad first-pass `rg`/file
inventory exploration with deterministic Cargo, owner, dependency public-api,
symbol, import, text, pattern, docs, tests, and ingest views.
`search --json` emits the shared
`agent.semantic-protocols.semantic-search-packet` envelope with
`languageId=rust`, `providerId=rs-harness`, `binary=rs-harness`, and
`namespace=agent.semantic-protocols.semantic-language`,
`method=search/<view>` for tools, while `check --changed` and `check --full`
provide the RFC validation entrypoints.
`search --view seeds` emits only prioritized `next=` follow-up axes, defaults
to 8 seeds, and accepts `--seeds N` when a caller needs a tighter packet. Seed
and detail views merge equivalent package headers and same-kind seed rows so
large workspace packets stay subagent-friendly.
`agent install` and `agent doctor` manage client-specific integration assets
without assuming a specific agent client. Codex installs project-local
`.codex/config.toml`, `.codex/harness-policy.json`, and
`.codex/skills/rs-harness/SKILL.org`.
Global CLI install helpers live in `Justfile`: `just install-bin-macos` installs
to `/opt/homebrew/bin`, `just install-bin-linux` installs to `/usr/local/bin`,
and both accept an optional prefix argument.
`agent doctor --json` emits the semantic-language registry with callable methods
and method descriptors.
`search docs` and `search docs-use` prefer native parser facts for local public
API shape, including compact signature, parameter, receiver, return, async,
unsafe, and error-boundary fields.
`search api` returns only that parser-native API shape, while
`search public-external-types` joins parser-native public API type facts with
Cargo dependency facts to find dependency types exposed through public
signatures. Versioned docs/API queries mark explicit external versions with
`source=registry-source` and `versionScope=external` instead of attributing
current workspace facts to that version. Dependency-prefixed docs/API queries
also report `source=registry-source` unless local parser facts exist for a
workspace/path dependency source. Public inherent impl methods and trait impl
methods are included in the API projection, so receiver, impl type, and trait
path are available without falling back to text search.
The `public-anyhow-result` and `public-error-boundary` pattern recipes use the
same native parser return facts instead of grepping for result type text.

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

When an owner handles several public surfaces, use `RustVerificationApiPathBaseline`
instead of making the whole owner louder. A path baseline attaches a compact
`METHOD:/path` fact to the verification fingerprint, so a receipt for
`POST:/v1/orders` does not clear `GET:/v1/orders/{id}`. This is still
library-first configuration: the harness does not guess framework routes or
force every API into stress, security, or performance. The Agent declares the
path responsibilities or exact task kinds, runs the matching skill, then records
a receipt or waiver for that path fingerprint.

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
    RustOwnerResponsibility, RustVerificationApiPathBaseline, RustVerificationPhase,
    RustVerificationProfileHint, RustVerificationRequirement, RustVerificationSkillBinding,
    RustVerificationSkillDescriptor, RustVerificationTaskContract, RustVerificationTaskKind,
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
            )
            .with_rationale("this slice changes route authorization"),
    )
    .with_verification_api_path_baseline(
        RustVerificationApiPathBaseline::new("src/api.rs", "POST", "/v1/orders")
            .with_task_kinds([
                RustVerificationTaskKind::Security,
                RustVerificationTaskKind::Performance,
            ])
            .with_rationale("order creation has tenant authz and latency SLO evidence"),
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
