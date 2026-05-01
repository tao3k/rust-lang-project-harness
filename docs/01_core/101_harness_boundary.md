# Harness Boundary

`rust-lang-project-harness` owns project-level Rust harness behavior:

1. discovering conventional Rust project paths
2. parsing Rust files with native Rust syntax
3. emitting deterministic findings from small rule packs
4. rendering compact diagnostics for humans and repair-oriented agents
5. exposing assertion helpers that can be mounted in Cargo test targets

The package is deliberately library-like. It does not know about a specific
workspace, crate family, or CI provider. Callers pass a project root or explicit
paths, then decide whether to assert, render, or inspect the report.

Project-root runners execute the full policy surface. By default they cover Rust
code under the crate's `src/`, `tests/`, `examples/`, and `benches/` roots, plus
root `build.rs` when it exists, so the harness is package-level rather than
source-only.
Explicit-path runners have no project scope and therefore act as focused Rust
syntax probes. See
[`Runner Modes`](../03_features/202_runner_modes.md) for the exact split.

Native Rust syntax parsing is a core substrate, not a rule-local detail. Policy
packs must parse Rust through `src/parser/`; rule modules consume parsed modules,
source locations, source metrics, and native syntax facts from that layer
instead of calling `syn::parse_file` or duplicating source scans themselves.
This keeps comments, strings, macro text, and real Rust AST nodes separated
before policy logic runs.

Cargo manifest policy follows the same boundary discipline for non-Rust input:
`Cargo.toml` is parsed into facts under `src/parser/` before project discovery
or project-policy rules inspect workspace members, test targets, or harness
dependencies. Comments and prose in the manifest do not count as dependency
evidence.

Custom project source roots configured through `RustHarnessConfig` are treated
as source ownership roots by project, modularity, and agent policy packs.
Within those roots, `lib.rs` is treated as a crate facade: it should declare
external modules and re-export public API, while macro implementations and other
owned logic live in leaf modules. `mod.rs` has the same special-file treatment
at module boundaries: it should expose declarations and re-exports, not own
implementation bodies.
Binary entrypoints have the matching adapter contract: `src/main.rs` and
`src/bin` files should contain imports and `fn main`, while CLI parsing and
execution logic live in owned modules.
Root `build.rs` has the same thin-entrypoint contract for Cargo build-script
logic: keep imports and `fn main` there, and move larger build behavior behind a
build dependency.

The path-clarity surface is also project-scoped. Modularity policy consumes
native Rust `use` tree facts from the parser to reject `super::super` owner
escapes and all glob imports, including `use super::*`. Agent advice reports
repeated namespace segments across the default package surface, including test
helpers and ordinary Rust file stems.

## Self-Apply Contract

The package is also self-hosted by its own default policy. The library target
mounts `rust_project_harness_cargo_test_gate!` from `src/self_policy.rs`, and
the root Cargo test target mounts `rust_project_harness_gate!` directly. Together
they prove both supported cargo-test embedding modes while keeping policy
changes subject to the same gate downstream projects consume.

New source-backed test modules should stay under `tests/unit` and be mounted
with `#[path]`. New root Cargo test targets should mount the project harness
gate themselves, or they will be reported by `RUST-PROJ-R006`.
Harness-enabled library crates should also mount
`rust_project_harness_cargo_test_gate!` from a `#[cfg(test)]` source module, or
they will be reported by `RUST-PROJ-R009`; root test targets alone do not run
under `cargo test --lib`.
Root Cargo test targets are thin aggregates: they may mount the harness gate and
external suite modules, but test bodies and helpers belong in suite files under
`tests/unit`, `tests/integration`, or a documented custom suite.
Use explicit `#[path = "suite/file.rs"]` module mounts from root targets; bare
`mod helper;` is intentionally rejected because it relies on implicit Rust file
lookup at the test root.

## Layout Exceptions

Projects can document intentional test-layout exceptions in
`tests/rust-project-harness-rules.toml`:

```toml
[tests]
allowed_root_files = [
  { name = "custom_gate.rs", explanation = "explicit harness aggregate" },
]
allowed_directories = [
  { name = "contract", explanation = "contract fixtures mounted by a root gate" },
]
```

Entries without a non-empty `explanation` are ignored. The exception file is for
auditable project structure, not for silent allowlists.

## Blocking And Advice

`Warning` and `Error` findings are blocking by default. `Info` findings are
advisory. The `AGENT-*` policy pack is intentionally `Info`-only so agents see
LLM-friendly repair hints without turning every style concern into a gate.

## Non-Goals

The first standalone version does not move the full scenario framework,
performance gate kernel, or workshop-specific contract runner from
`xiuxian-testing`. Those can become separate packages later if the boundary
needs them.
