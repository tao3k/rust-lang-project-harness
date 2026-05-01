---
name: rust-verification-profile
description: "Use when configuring RustVerificationProfileHint, verification task mappings, receipts, waivers, or verification-policy tests."
---

# Rust Verification Profile

Use this skill when editing the verification planner, config surface, receipts,
waivers, or documentation for verification responsibilities.

## Authority Order

Verification is library-first and parser-native. The authority order is:

1. Parser facts from the Rust syntax and reasoning tree.
2. Current external skill receipts tied to the task fingerprint.
3. Complete waivers tied to the task fingerprint.
4. Library config profile hints.
5. LLM prose.

Profile hints are useful declarations, not facts. If a hint conflicts with
parser evidence, the planner should keep a `responsibility_review` task active.

## Profile Rules

- `RustVerificationProfileHint::new(path, responsibilities)` maps an owner path
  to responsibility labels.
- Responsibility mappings choose default task kinds. The available task
  families currently include stress, performance, chaos, security, regression,
  and responsibility-review.
- `with_task_kinds([...])` is an owner-local override. It wins only for that
  owner.
- `without_verification_tasks()` is an owner-local suppression. It must be
  deliberate and explained when it changes the responsibility-derived default.
- `with_task_contract(kind, contract)` overrides the contract only for that
  owner and task kind.
- `with_rationale(...)` is required when an owner-local override adds, removes,
  or suppresses task kinds relative to the responsibility mapping.
- A task contract attached to a task kind that is not effective for that owner
  should produce `responsibility_review`; otherwise it becomes invisible drift.

## Receipts And Waivers

- Receipts and waivers clear tasks by exact fingerprint.
- Changing owner path, task kind, required evidence, parser evidence, or profile
  evidence changes the fingerprint and invalidates stale clearance.
- A passed receipt means the external skill ran and produced all required
  evidence.
- A waiver means the task is intentionally out of scope. It needs owner, reason,
  and expiry.
- Incomplete waivers and stale receipts should stay visible in compact output as
  resolution feedback.

## Test Expectations

- Add focused unit tests for mapping and owner-local override behavior.
- Add snapshots for compact verification output changes.
- Keep tests folder-first when a config suite grows beyond one responsibility.
- Preserve the single-rule override behavior: a specific rule or task override
  must still win after any pack-level helper expands defaults.
