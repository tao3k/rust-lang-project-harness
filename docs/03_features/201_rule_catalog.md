# Rule Catalog

The harness exposes deterministic rule metadata through compact catalog
functions:

- `rust_rule_pack_descriptors()`
- `rust_syntax_rules()`
- `rust_project_policy_rules()`
- `rust_modularity_rules()`
- `rust_agent_policy_rules()`

## Default Rule Packs

Default project execution runs these packs:

1. `rust.syntax`
2. `rust.project_policy`
3. `rust.modularity`
4. `rust.agent_policy`

The package self-applies this same default execution order in its own tests.
Policy drift should be repaired in the crate before the rule catalog is treated
as ready for downstream use.

`rust_rule_pack_descriptors()` returns the same pack order with stable pack ids,
version labels, searchable domains, and default modes. The first three packs are
`blocking`; `rust.agent_policy` is `advisory`.

## Blocking Rules

- `RUST-SYN-R001`: Rust source must parse through `syn`
- `RUST-PROJ-R001`: root test file must be an explicit harness entry point
- `RUST-PROJ-R002`: directory under `tests/` must be an allowed suite directory
- `RUST-PROJ-R003`: source tests must be externalized instead of inline
- `RUST-PROJ-R004`: external test mount must point to an existing `tests/unit` file
- `RUST-PROJ-R005`: large test leaf should split into a folder-first suite
- `RUST-PROJ-R006`: Cargo test target must mount the project harness gate
- `RUST-PROJ-R007`: root Cargo test target should stay a thin harness aggregate
- `RUST-PROJ-R008`: root Cargo test target modules should use explicit suite `#[path]` mounts
- `RUST-PROJ-R009`: harness-enabled library target must mount the cargo-test gate for `cargo test --lib`
- `RUST-MOD-R001`: `mod.rs` should stay interface-only with external module declarations and re-exports
- `RUST-MOD-R002`: oversized source file should split by responsibility, including private implementation piles
- `RUST-MOD-R003`: native `use` trees containing `super::super` should move behind a clearer owner boundary
- `RUST-MOD-R004`: `lib.rs` should stay a crate facade with module declarations and re-exports only
- `RUST-MOD-R005`: `src/main.rs` and `src/bin` entrypoints should stay thin
- `RUST-MOD-R006`: root `build.rs` should stay a thin Cargo build-script entrypoint
- `RUST-MOD-R007`: a module owner should not have both `foo.rs` and `foo/mod.rs`
- `RUST-MOD-R008`: source modules should not hide implementation in inline `mod name { ... }` blocks
- `RUST-MOD-R009`: scanned source files must be reachable from a crate or binary module tree
- `RUST-MOD-R010`: Rust glob imports should be replaced with explicit owner imports

`lib.rs`, `mod.rs`, `src/main.rs`, `src/bin` entrypoints, and root `build.rs`
are special Rust ownership files. They are treated as
facades/interfaces/adapters instead of implementation owners, so they receive
targeted modularity checks before generic source-size policy matters.
The module tree also has one source-layout owner per module: `foo.rs` and
`foo/mod.rs` should not coexist because that leaves repair agents with two
competing files for the same logical owner.
Inline implementation modules collapse the reasoning tree back into a single
file, so regular source modules should use external file-backed child modules.
Files that are not reachable through `mod` declarations are also blocked:
they look like source to search tools but are invisible to the Rust module tree
and therefore unsafe for agents to treat as live owners.

Root Cargo test target files under `tests/*.rs` are also special entrypoints.
They should mount the harness gate and external suite modules, while test bodies
and helpers live under `tests/unit`, `tests/integration`, or another documented
suite directory. Module mounts from those root targets must use explicit
`#[path = "suite/file.rs"]` attributes so Rust's implicit module lookup does not
create unclear root-level test structure.

Library crates have one additional cargo-test escape hatch: `cargo test --lib`
does not execute root test targets under `tests/*.rs`. `RUST-PROJ-R009` closes
that path for harness-enabled projects by requiring a source-tree cargo-test
mount. The harness-enabled decision comes from parsed Cargo manifest dependency
tables or native Rust gate invocations, not comment or string matches. The mount
normally looks like:

```rust
#[cfg(test)]
rust_lang_project_harness::rust_project_harness_cargo_test_gate!();
```

The mount should live in `src/lib.rs` or in a source module declared by
`src/lib.rs`, so both `cargo test` and `cargo test --lib` execute project
policy.

## Agent Advice Rules

`AGENT-*` rules are `Info` findings. They are designed as repair hints for LLMs
and are not blocking by default.

- `AGENT-R001`: public module surface lacks an intent doc
- `AGENT-R002`: public item lacks a doc comment
- `AGENT-R003`: namespace path repeats a segment, including ordinary Rust file stems
- `AGENT-R004`: public item name appears in multiple modules
- `AGENT-R005`: facade re-exports too many names without a tighter owner surface
- `AGENT-R006`: public module name is a generic bucket such as `utils`, `common`, `helpers`, or `shared`
- `AGENT-R007`: source module file or directory path uses a generic bucket segment
- `AGENT-R008`: branch module declares multiple child modules without a reasoning-tree intent doc

## Rendered Diagnostic Policy

Rendered findings intentionally avoid large JSON payloads. The primary repair
surface is compact text:

1. stable rule id
2. source location
3. highlighted source line when available
4. short source pointer label
5. one `Help:` line from the concrete finding summary
6. one `Contract:` line from the rule requirement

`render_rust_project_harness()` includes advice by default. A report with only
`Info` findings is still clean, but its advice remains visible. Use
`render_rust_project_harness_advice()` when a caller wants only the non-blocking
repair hints.

Structured consumers should use `render_rust_project_harness_json()` or the
serializable `RustHarnessReport` for JSON output instead of parsing the compact
text render.

The compact text and JSON render contracts are covered by repository snapshots
under `tests/unit/snapshots`. Every `RUST-MOD-*` policy also has an
`insta` compact-diagnostic snapshot generated from a real harness fixture, so
changes to structural `Help:` and `Contract:` wording are reviewed per rule.
Every `AGENT-*` policy has the same snapshot treatment for LLM-facing advice,
including multi-finding ambiguity cases such as duplicated public names.

## Path Clarity Policy

Path clarity rules follow Rust syntax and project scope instead of raw text
searches. `RUST-MOD-R003` and `RUST-MOD-R010` consume native `use` tree facts
from `src/parser/`, so grouped uses such as `use super::{super::Owner}` and
glob imports such as `use super::*` are caught while comments and strings are
ignored.

`AGENT-R001`, `AGENT-R002`, `AGENT-R004`, `AGENT-R005`, `AGENT-R006`, and
`AGENT-R008` consume native facts from `src/parser/`, including file-level inner
doc attributes, public names, public item doc attributes, public re-export
groups, and external child module declarations. `AGENT-R003` evaluates the
default package harness surface, including `src/` and `tests/`. It treats normal
Rust file stems as namespace segments, so both `src/domain/domain.rs` and
`tests/unit/unit/helper.rs` produce advisory path clarity findings. `AGENT-R004`
separately reports duplicated public item names across source modules as
non-blocking ambiguity advice. `AGENT-R006` catches public generic bucket
modules such as `pub mod utils;`; those names are often where LLM-generated code
loses a real owner boundary without violating Rust syntax, rustfmt, or Clippy.
`AGENT-R007` catches the same drift at the file system level, such as
`src/helpers.rs` or `src/common/mod.rs`, even when the module is private.

## Reasoning Tree Policy

The harness treats a Rust project as a reasoning tree for agents: crate
facades and branch modules point to owner modules, and owner modules point to
leaf implementation files. `RUST-MOD-R008` keeps those branches file-backed by
rejecting inline source modules outside special entrypoints and `#[cfg(test)]`
test modules. `RUST-MOD-R009` then verifies parser-owned module-tree facts: a
scanned source file must be reachable from crate roots or binary roots through
external `mod` declarations, explicit `#[path]` mounts, or literal `include!`
source shards. `AGENT-R008` adds non-blocking advice when a branch file
declares multiple children without a `//!` intent doc, because agents need a
local navigation summary before they choose which subtree to edit.
