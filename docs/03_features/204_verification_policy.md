# Verification Policy

Verification policy is a parser-native task contract for external Agent skills.
It does not run stress, chaos, security, or regression tools by itself. The
harness decides when a task is structurally relevant, attaches structured
evidence requirements, renders a compact owner-level reminder for the Agent, and
accepts a receipt or waiver that removes the reminder for the current
parser-fact fingerprint.

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
    plan_rust_project_verification_with_config, render_rust_verification_plan,
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

Compact text is grouped by owner path. If one owner needs stress, chaos, and
security verification, the renderer emits one `[verify] owner.rs` block with
task-specific lines instead of three repeated owner cards.

## Configurable Surface

Verification config stays library-first. It does not introduce CLI flags or
TOML precedence. Embedding projects can adjust the verification contract through
`RustHarnessConfig` or `RustVerificationPolicy`.

There are two configurable layers:

- Responsibility mapping: choose which task kinds a declared responsibility
  triggers. Mapping a responsibility to an empty set suppresses the default
  task for that responsibility.
- Task contract: override the phase, receipt contract, and structured evidence
  keys for a task kind.

```rust
use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationPhase, RustVerificationProfileHint,
    RustVerificationRequirement, RustVerificationTaskContract, RustVerificationTaskKind,
    default_rust_harness_config,
};

let config = default_rust_harness_config()
    .with_verification_profile_hint(RustVerificationProfileHint::new(
        "src/api.rs",
        [RustOwnerResponsibility::PublicApi],
    ))
    .with_verification_responsibility_task_kinds(
        RustOwnerResponsibility::PublicApi,
        [RustVerificationTaskKind::Security],
    )
    .with_verification_task_contract(
        RustVerificationTaskKind::Security,
        RustVerificationTaskContract::new(
            RustVerificationPhase::BeforeRelease,
            "security skill must report tenant authz probes for this fingerprint",
            [RustVerificationRequirement::new(
                "tenant_authz",
                "tenant authz probe result",
            )],
        ),
    );
```

## Task Families

- `stress`: high-concurrency load, p50/p99/p999, SLA break detection
- `chaos`: dependency kill, delay, packet loss, degradation, recovery
- `security`: common attack-surface probes and authorization-boundary checks
- `regression`: architecture drift checks such as branch growth, owner fan-out,
  and cycle health
- `responsibility_review`: config/profile does not match parser facts

Verification tasks are not harness findings. A finding means a policy violation
inside the Rust project. A verification task means an external Agent skill should
produce evidence before the task is considered handled. Each task also carries
`required_evidence` for structured consumers. For example, stress verification
requires keys such as `p50`, `p99`, `p999`, `load_steps`, and `sla_result`.

## Receipt And Waiver Lifecycle

Each task has a stable `fingerprint` derived from the task kind, owner path,
structured requirement keys, and parser/profile evidence. When the code,
responsibility evidence, or verification contract changes, the fingerprint
changes and old receipts no longer clear the task.

Use a receipt when the external skill ran:

```rust
use rust_lang_project_harness::{RustVerificationReceipt, RustVerificationTaskKind};

let receipt = RustVerificationReceipt::passed(task.fingerprint.clone(), RustVerificationTaskKind::Stress);
```

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
[verify] src/api.rs
   |owner: src/api
   |stress: pending phase=after_unit_tests_pass fingerprint=rustv:...
   |why: stress=profile declares public or latency-sensitive surface
   |requires: stress=p50,p99,p999,load_steps,sla_result
   |fact: stress.profile=public_api,latency_sensitive
   |contract: stress=stress skill must report p50/p99/p999, load steps, and SLA result for this fingerprint
```

When there are no active tasks, the compact string is empty.
