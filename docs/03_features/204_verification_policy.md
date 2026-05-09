# Verification Policy

Verification policy is a parser-native task contract for external Agent skills.
It does not run cargo bench, Criterion, Divan, iai-callgrind, stress, chaos,
security, or regression tools by itself. The harness decides when a task is
structurally relevant, attaches structured evidence requirements, renders a
compact owner-level reminder for the Agent, and accepts a receipt or waiver that
removes the reminder for the current parser-fact fingerprint.

The authority order is:

1. parser facts from the Rust syntax/reasoning tree
2. external skill receipts tied to a current task fingerprint
3. complete waivers tied to a current task fingerprint
4. library config profile hints
5. LLM prose

Profile hints are useful, but they are not facts. If a hint says an owner is
pure domain logic while the parser sees external imports or local owner
dependencies, the harness emits a `responsibility_review` task instead of
trusting the hint.

## Library API

The surface is library-first:

```rust
use std::path::Path;

use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationProfileHint, default_rust_harness_config,
    build_rust_verification_performance_index, build_rust_verification_report_bundle,
    plan_rust_project_verification_with_config, render_rust_verification_performance_index_json,
    render_rust_verification_plan, render_rust_verification_report_artifact_json,
    render_rust_verification_report_bundle_json, render_rust_verification_skill_contracts,
};

let config = default_rust_harness_config().with_verification_profile_hint(
    RustVerificationProfileHint::new(
        "src/api.rs",
        [
            RustOwnerResponsibility::PublicApi,
            RustOwnerResponsibility::LatencySensitive,
        ],
    ),
);

let plan =
    plan_rust_project_verification_with_config(Path::new("."), &config).expect("plan");
let compact = render_rust_verification_plan(&plan);
let contract_tree = render_rust_verification_skill_contracts(&plan);
let perf_index = build_rust_verification_performance_index(&plan);
let perf_json = render_rust_verification_performance_index_json(&perf_index).expect("json");
let report_bundle = build_rust_verification_report_bundle(&plan);
let report_manifest_json = render_rust_verification_report_bundle_json(&plan).expect("json");
let performance_artifact_json =
    render_rust_verification_report_artifact_json(&plan, "performance_index_json")
        .expect("json")
        .expect("active performance artifact");
```

Relative profile paths are matched against parser-known modules. In a single
package, `src/api.rs` is package-relative. In a workspace, `src/api.rs` can match
that path in every member package, while `crates/api/src/api.rs` targets a
single member from the workspace root. This keeps compact renders useful for
multi-package projects and keeps task fingerprints package-aware when different
members have the same owner path.

The compact renderer only prints active tasks. A passed receipt or complete
waiver makes the matching task disappear from this channel. Structured callers
can keep the full task state through `render_rust_verification_plan_json()`.
If a receipt or waiver is present but cannot clear the task, the active task
keeps rendering with a `resolution:` line so the Agent knows what still needs to
be fixed.

Compact text is grouped by owner path. If one owner needs stress, performance,
chaos, and security verification, the renderer emits one `[verify] owner.rs`
block with task-specific lines instead of repeated owner cards.

Active verification tasks also create durable report obligations. The compact
renderer emits one `[verify-report]` block that tells the Agent which harness
renderers should be persisted by the embedding project. Every active plan
requires `verification_plan_json` through `render_rust_verification_plan_json`.
If any active task is `performance`, the plan also requires
`performance_index_json` through
`build_rust_verification_performance_index` plus
`render_rust_verification_performance_index_json`. These obligations are also
serialized in `RustVerificationPlan::report_obligations`, including the active
task kinds, fingerprints, renderer, reason, and suggested artifact filename.
This keeps the hot prompt compact while making the durable metrics surface an
upstream harness contract instead of a downstream convention.

Agents and CI integrations do not need to reconstruct those artifact contracts
manually. `build_rust_verification_report_bundle(&plan)` and
`render_rust_verification_report_bundle_json(&plan)` produce a small manifest,
not one large all-in-one payload. The manifest lists modular artifacts,
template metadata, trace guidance, runtime budgets, and task fingerprints.
Call `render_rust_verification_report_artifact_json(&plan, key)` to render one
artifact at a time, such as `verification_plan_json` or `performance_index_json`.
This keeps performance evidence, plan state, and future report families
separately persistable and separately comparable.

The manifest also classifies each artifact by persistence target.
`performance_index_json` defaults to `source_baseline` because benchmark
ownership, missing evidence keys, and trace budgets are useful long-term
optimization signals. `verification_plan_json` defaults to `runtime_cache`
because it is verbose, path-heavy run state. Embedding projects can override
those defaults through `RustVerificationReportOptions`, but the upstream
contract makes the source-vs-cache choice explicit so Agents do not commit every
large report by accident.

## Configurable Surface

Verification config stays library-first. It does not introduce CLI flags or
TOML precedence. Embedding projects can adjust the verification contract through
`RustHarnessConfig` or `RustVerificationPolicy`.

There are seven configurable layers:

- Profile index: derive a compact owner-profile draft from parser facts before
  the Agent writes config. It renders only missing or drifting hints, then goes
  quiet after matching `RustVerificationProfileHint` entries exist.
- API path baseline: declare one public `METHOD:/path` surface under a
  parser-known owner. This makes receipts and waivers path-scoped, so resolving
  one route does not mute a different route in the same module.
- Responsibility mapping: choose which task kinds a declared responsibility
  triggers. Mapping a responsibility to an empty set suppresses the default
  task for that responsibility.
- Global task contract: override the phase, receipt contract, and structured
  evidence keys for a task kind across the project.
- Owner profile override: let one owner choose its exact task kinds and
  contracts. This is the Agent-facing layer for declaring that a concrete
  module needs stress, performance, security, chaos, regression, or no external
  skill in the current responsibility boundary.
- Skill binding: bind a task kind to a configured Agent skill adapter. This is
  the quiet dispatch layer; once present, compact output no longer repeats the
  skill manual every run.
- Skill descriptor: define the compact execution contract for a configured
  skill adapter. Descriptors stay out of default verification output and expand
  only through `render_rust_verification_skill_contracts(&plan)`.

```rust
use std::path::Path;

use rust_lang_project_harness::{
    build_rust_verification_profile_index, render_rust_verification_profile_index,
};

let index = build_rust_verification_profile_index(Path::new("."))?;
let compact_profile_advice = render_rust_verification_profile_index(&index);
let suggested_hints = index.active_profile_hints();
```

The profile index is not a human audit report. It is the Agent's configuration
draft for diverse crates: owner path, suggested responsibilities, implied task
kinds, and parser evidence such as public items, local owner dependencies, and
Cargo dependency roots. When parser facts find owners but no
`RustVerificationProfileHint` has been configured yet, the compact render starts
with one short configuration reminder:

```text
[verify-profile] profile_hints
   |state: missing_profile_config
   |action: configure RustVerificationProfileHint entries
   |candidates: 4
```

That reminder is intentionally not a skill manual. It tells an Agent which
library config surface to populate; after any profile hints exist, the reminder
disappears and only missing or drifting owner blocks remain. Cargo roots are
native parser facts:
dependency key, Rust import root, `package = "..."` rename, optional flag,
features, target table, and dev/build table are preserved before policy runs.
The upstream harness does not decide that a third-party crate is persistence,
security, network, or performance-sensitive. Projects declare those semantics
through `RustVerificationDependencySignal`, and the profile index only applies
them after a `use` root has matched a Cargo dependency fact. `active_profile_hints()`
gives the Agent code-level config material; after those hints are supplied
through `RustHarnessConfig`, the compact profile advice becomes empty.

Cargo parsing exists here to help Agents manage modern Rust projects, not to
turn the harness into a Cargo replacement. The parser records the dependency
facts an Agent needs to make good configuration decisions: which dependency key
is imported, whether it is renamed with `package = "..."`, whether it comes from
normal/dev/build/target tables, whether it is inherited from workspace
dependencies, and which dependency features are explicitly requested. Those
facts let the Agent ask a precise question such as "does this owner cross a
persistence or network boundary?" without scanning massive manifests or guessing
from crate names. Resolved dependency graphs, feature unification, platform
evaluation, and transitive supply-chain analysis stay out of this milestone
until a policy genuinely needs those facts.
The compact evidence also separates `configured_dependency_roots` from
`unconfigured_dependency_roots`. Configured roots have matched a
`RustVerificationDependencySignal` and can affect suggested responsibilities.
Unconfigured roots stay as facts only: they tell the Agent where a project-owned
decision may be missing, without turning every third-party crate into a chaos,
security, or performance requirement by default.

The index prefers reasoning-tree owner branches over individual leaf files.
Branch owners aggregate child-module signals such as public route surfaces,
configured dependency signals, and Rust standard-library boundary facts.
Standalone leaf modules still get candidates when no parent owner exists. Crate
root facades that only re-export owner APIs do not shadow the more precise owner
profile. Source paths identify owners; they do not by themselves imply security,
performance, persistence, or availability responsibility.

```rust
use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationApiPathBaseline, RustVerificationDependencySignal,
    RustVerificationPhase, RustVerificationProfileHint, RustVerificationRequirement,
    RustVerificationSkillBinding, RustVerificationSkillDescriptor,
    RustVerificationTaskContract, RustVerificationTaskKind, default_rust_harness_config,
};

let config = default_rust_harness_config()
    .with_verification_dependency_signal(RustVerificationDependencySignal::new(
        "arrow-flight",
        [
            RustOwnerResponsibility::ExternalDependency,
            RustOwnerResponsibility::AvailabilityCritical,
        ],
    ))
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
        ),
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

Owner-local config wins over global defaults only for that profile. Use
`RustVerificationProfileHint::with_task_kinds([...])` when the Agent can name
the exact verification skill families for an owner. Use
`RustVerificationProfileHint::without_verification_tasks()` when the owner is
deliberately out of scope for external stress, performance, chaos, security, or
regression evidence in this slice. Receipts and waivers still clear tasks through
fingerprints, so changing the owner-local task kinds or contracts invalidates
stale evidence automatically.

API path baselines are more precise than owner profiles. Use
`RustVerificationApiPathBaseline::new("src/api.rs", "GET", "/v1/search")`
when the responsibility belongs to one route or command path, then add
`with_responsibility(...)` or `with_task_kinds(...)` to define the expected
security, performance, stress, chaos, or regression skill families. Compact
verification output keeps the short `api=GET:/v1/search` token on the task line,
including when a skill binding is configured and the long fact block is omitted.
Because that API path evidence participates in the task fingerprint, a passed
receipt or complete waiver clears only that path-level obligation.

Owner-local overrides are allowed to differ from the responsibility-derived
default, but they must explain the responsibility boundary. If a profile adds,
removes, or suppresses task kinds without `with_rationale(...)`, the planner
keeps a `responsibility_review` task active instead of silently trusting the
configuration. The same review task is emitted when an owner-local contract is
attached to a task kind that is not effective for that owner.

Skill bindings are intentionally separate from task contracts. A contract says
what evidence is required. A binding says the project already has a skill or
adapter that knows how to produce that evidence. This avoids loading large skill
Markdown on every run. If a binding is absent, compact text includes the
fallback contract lines. If a binding is present, compact text emits only the
task line with `skill=<id>` while JSON keeps `required_evidence` and
`required_receipt` for structured consumers. The binding label participates in
the task fingerprint, so switching adapters invalidates stale receipts.
Snapshot tests for configured skill triggers should stay compact-first as well:
the agent-facing baseline is short text, while JSON is asserted only as a
secondary structured contract.

Skill descriptors are intentionally separate from bindings. A binding routes
the task to `skill=<id>@<adapter>`. A descriptor explains that adapter as a
small reasoning-tree node: tool, command template, pass/fail standard, inputs,
and receipt fields. The compact verification renderer only emits
`contract_ref=<id>@<adapter>` when a descriptor exists; it does not inline the
descriptor. Agents expand the reference through
`render_rust_verification_skill_contracts(&plan)` only when they need dispatch
details. Descriptor material participates in the task fingerprint, so changing
the adapter command, threshold standard, or expected receipt fields forces a
fresh receipt instead of silently clearing the old task.
Descriptor expansion is also active-task scoped: once a matching receipt or
complete waiver clears the task, the compact verification render and the
optional descriptor render both go quiet.

Descriptors also keep stress and Rust-native performance separate. `k6` belongs
to the `stress` family for service-boundary pressure, p50/p99/p999, and SLA
evidence. Criterion, Divan, and iai-callgrind belong to the `performance`
family for Rust code-level benchmark, allocation, instruction, cache, and
profiling evidence.

## Task Families

- `stress`: high-concurrency load, p50/p99/p999, SLA break detection
- `performance`: Rust-native benchmark and allocation regression evidence,
  such as `cargo bench`, Criterion, Divan, iai-callgrind, flamegraph, or a
  project-owned benchmark command
- `chaos`: dependency kill, delay, packet loss, degradation, recovery
- `security`: common attack-surface probes and authorization-boundary checks
- `regression`: architecture drift checks such as branch growth, owner fan-out,
  and cycle health
- `responsibility_review`: config/profile does not match parser facts

Verification tasks are not harness findings. A finding means a policy violation
inside the Rust project. A verification task means an external Agent skill should
produce evidence before the task is considered handled. Each task also carries
`required_evidence` for structured consumers. For example, stress verification
requires keys such as `p50`, `p99`, `p999`, `load_steps`, and `sla_result`;
performance verification requires keys such as `benchmark_command`, `baseline`,
`regression_threshold`, `latency_or_throughput`, `allocation_profile`, and
`profile_artifact`.

## Receipt And Waiver Lifecycle

Each task has a stable `fingerprint` derived from the task kind, owner path,
structured requirement keys, and parser/profile evidence. When the code,
responsibility evidence, or verification contract changes, the fingerprint
changes and old receipts no longer clear the task.

Use a receipt when the external skill ran:

```rust
use rust_lang_project_harness::{RustVerificationReceipt, RustVerificationTaskKind};

let receipt = RustVerificationReceipt::passed(
    task.fingerprint.clone(),
    RustVerificationTaskKind::Performance,
)
.with_evidence("benchmark_command", "cargo bench --bench parser_hot_path")
.with_evidence("baseline", "main@b0a8a7a")
.with_evidence("regression_threshold", "5%")
.with_evidence("latency_or_throughput", "-1.4% latency")
.with_evidence("allocation_profile", "allocs/op unchanged")
.with_evidence(
    "profile_artifact",
    "target/criterion/parser_hot_path/report/index.html",
)
.with_evidence_uri("target/criterion/parser_hot_path/report/index.html")
.with_observed_at("2026-05-01T20:00:00Z");
```

Receipt evidence is copied into the matching task as `receipt_evidence`.
Compact output still disappears when the task is satisfied, because the Agent no
longer needs a reminder. Structured callers can keep searching the JSON state
for the command, baseline, threshold, metric delta, allocation profile,
artifact URI, and observed timestamp.

Use `build_rust_verification_performance_index(&plan)` when a caller needs a
performance-specific retrieval surface. It returns only `performance` task
records, including pending, failed, satisfied, and waived states. Each record is
keyed by parser-owned package path, owner path, owner namespace, fingerprint,
state, skill binding, required evidence keys, and receipt evidence. Callers can
query by owner, package, state, or receipt evidence key, and each record can
compute `missing_receipt_evidence_keys()` so a failed or partial receipt tells
the Agent exactly which benchmark/profiling facts are still absent. This is the
intended performance-status lane: human-readable reminders stay quiet, while
benchmark state remains traceable for CI indexes, dashboards, or future
reasoning-tree retrieval.

The verification plan's `performance_index_json` report obligation is emitted
while any performance task remains active. A downstream workspace can persist
the rendered index under its own contract snapshot or CI artifact location, but
the Agent-facing reminder to create that artifact comes from the harness plan.

Use a waiver when the task is intentionally out of scope for the current work:

```rust
use rust_lang_project_harness::RustVerificationWaiver;

let waiver = RustVerificationWaiver::new(
    task.fingerprint.clone(),
    "platform",
    "covered by upstream gateway test for this release",
    "2026-06-01",
);
```

Waivers must carry an owner, reason, and expiry string. In this stage the harness
checks completeness and fingerprint identity; expiry interpretation remains owned
by the embedding project.
An incomplete waiver does not clear the task; compact output records the missing
fields as resolution feedback.

## Agent-First Output

The compact verification renderer is not a human audit header. It does not print
package counts, source roots, success summaries, or empty sections. It starts at
the active obligation:

```text
[verify-profile] src/api.rs
   |owner: src/api
   |state: missing_profile
   |suggest: public_api,external_dependency,availability_critical
   |tasks: stress,chaos
   |fact: public_items=1
   |fact: network_roots=axum::Router
```

Profile advice appears before task planning when the Agent still needs to
configure crate-specific responsibilities. It also stays compact: no package
counts, no empty sections, and no long skill text.

```text
[verify] src/api.rs
   |owner: src/api
   |stress: pending phase=after_unit_tests_pass fingerprint=rustv:...
   |why: stress=profile declares public API or integration surface
   |requires: stress=p50,p99,p999,load_steps,sla_result
   |fact: stress.profile=public_api
   |contract: stress=stress skill must report p50/p99/p999, load steps, and SLA result for this fingerprint
```

When there are no active tasks, the compact string is empty.

With a configured skill binding, the same active obligation is shorter:

```text
[verify] src/api.rs
   |owner: src/api
   |performance: pending phase=after_unit_tests_pass fingerprint=rustv:... skill=rust-verification-performance@criterion
```

With a configured skill descriptor, the default line still stays compact and
only adds a reference:

```text
[verify] src/api.rs
   |owner: src/api
   |stress: pending phase=after_unit_tests_pass fingerprint=rustv:... skill=rust-verification-stress@k6 contract_ref=rust-verification-stress@k6
```

Rust-native performance descriptors expand on demand as compact execution
contracts:

```text
[skill-contract] rust-verification-performance@criterion
   |tool: criterion
   |run: cargo bench
   |standard: statistical benchmark baseline detects runtime regression
   |inputs: bench_target,baseline,regression_threshold
   |pass: regression<=threshold
   |receipt: benchmark_command,baseline,regression_threshold,latency_or_throughput,allocation_profile,profile_artifact

[skill-contract] rust-verification-performance@divan
   |tool: divan
   |run: cargo bench
   |standard: sampled Rust benchmark summary stays within regression threshold
   |inputs: bench_target,baseline,regression_threshold
   |pass: median_or_mean_delta<=threshold
   |receipt: benchmark_command,baseline,regression_threshold,latency_or_throughput,allocation_profile,profile_artifact,samples,iters

[skill-contract] rust-verification-performance@iai-callgrind
   |tool: iai-callgrind
   |run: cargo bench
   |standard: instruction/cache/allocation metrics stay within regression threshold
   |inputs: bench_target,baseline,metric,regression_threshold
   |pass: metric_delta<=threshold
   |receipt: benchmark_command,baseline,regression_threshold,latency_or_throughput,allocation_profile,profile_artifact,instructions,cache_misses
```

The optional contract renderer can also expand a service-boundary stress
descriptor:

```text
[skill-contract] rust-verification-stress@k6
   |tool: k6
   |run: k6 run <script>
   |standard: scenarios define load shape; thresholds define pass/fail
   |inputs: script,target_url,scenario,thresholds
   |pass: exit=0,thresholds=pass
   |receipt: p50,p99,p999,load_steps,sla_result,artifact
```

Performance state has its own retrieval renderer. It is intentionally separate
from the default verification reminder and from descriptor expansion:

```text
[perf-state] src/api.rs
   |owner: src/api
   |state: satisfied phase=after_unit_tests_pass fingerprint=rustv:... skill=rust-verification-performance@criterion contract_ref=rust-verification-performance@criterion
   |receipt: passed (target/criterion/parser_hot_path/report/index.html)
   |observed_at: 2026-05-01T20:00:00Z
   |evidence: benchmark_command=cargo bench --bench parser_hot_path,baseline=main@...,regression_threshold=5%,latency_or_throughput=-1.4% latency,allocation_profile=allocs/op unchanged,profile_artifact=target/criterion/parser_hot_path/report/index.html
   |artifact: target/criterion/parser_hot_path/report/index.html
```

Partial failed receipts keep the same retrieval shape and add a compact missing
line instead of requiring the Agent to reopen the adapter contract:

```text
[perf-state] src/api.rs
   |owner: src/api
   |state: failed phase=after_unit_tests_pass fingerprint=rustv:... skill=rust-verification-performance@criterion contract_ref=rust-verification-performance@criterion
   |receipt: latency regression exceeded threshold
   |evidence: benchmark_command=cargo bench --bench parser_hot_path,regression_threshold=5%,latency_or_throughput=+11.2% latency
   |missing: baseline,allocation_profile,profile_artifact
```

The Rust-native descriptors follow the ecosystem split between Cargo's
benchmark entrypoint, statistical benchmarks, deterministic CI profiling, and
profiling-first optimization. See the official docs for
[`cargo bench`](https://doc.rust-lang.org/beta/cargo/commands/cargo-bench.html),
[Criterion.rs](https://bheisler.github.io/criterion.rs/book/index.html),
[Divan](https://docs.rs/divan/latest/divan/),
[iai-callgrind](https://docs.rs/iai-callgrind/latest/iai_callgrind/), and the
[Rust Performance Book benchmarking](https://nnethercote.github.io/perf-book/benchmarking.html)
and [profiling](https://nnethercote.github.io/perf-book/profiling.html)
chapters.
The built-in k6 descriptor follows Grafana k6's model: `k6 run <script>` is the
local execution command, scenarios describe load shape, and thresholds define
pass/fail behavior with a zero exit code on pass and nonzero exit code on
threshold failure. See the official k6 docs for
[running k6](https://grafana.com/docs/k6/latest/get-started/running-k6/),
[scenarios](https://grafana.com/docs/k6/latest/using-k6/scenarios/), and
[thresholds](https://grafana.com/docs/k6/latest/using-k6/thresholds/).
