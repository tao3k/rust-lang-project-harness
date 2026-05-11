# Harness Boundary

`rust-lang-project-harness` owns project-level Rust harness behavior:

1. discovering conventional Rust project paths
2. parsing Rust files with native Rust syntax
3. emitting deterministic findings from small rule packs
4. rendering compact diagnostics for repair-oriented agents
5. exposing assertion helpers that can be mounted in Cargo test targets

The package is deliberately library-like and Agent-facing. It does not know
about a specific workspace, crate family, or CI provider, and it does not assume
a human will read a long audit report before code is repaired. Callers pass a
project root or explicit paths, then decide whether to assert, render, or
inspect the report. The usual downstream loop is: mount the harness in Cargo,
let `cargo test` or a build script run it, and let the next coding Agent repair
the compact finding or configure an explicit project-local rationale.

The harness exists to reduce Agent search and keep long-lived Rust projects
structurally clean. It parses the project into package, module, owner, import,
child-edge, and dependency facts before policy runs. That parser-owned fact
layer gives an Agent a reasoning tree instead of a massive file list, while the
rule packs constrain common LLM drift such as broad scopes, oversized files,
unclear facades, hidden glob imports, primitive public APIs, missing verification
bindings, and partial harness configuration.

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
Root Cargo test targets follow that parser boundary too: conventional
`tests/*.rs` targets and manifest-declared `[[test]]` paths are collected and
parsed under `src/parser/` before `RUST-PROJ-R006`, `RUST-PROJ-R007`, and
`RUST-PROJ-R008` render findings.
Rust `#[path]` attributes are also resolved there, so project policy consumes
both the native attribute text and its normalized target path as parser facts.
Rust source path interpretation follows the same contract: namespace
components, repeated namespace branches, crate facades, `mod.rs` interfaces,
binary entrypoints, and root `build.rs` entrypoints are derived under
`src/parser/` before agent or modularity policies decide whether those facts are
acceptable.
Those lower-level facts are assembled into a parser-owned reasoning tree before
policy execution. The reasoning tree gives each parsed module a source role,
owner namespace, module-tree-root marker, import-root summary, local owner
dependency edges, declared child edges, and shadow/orphan reachability state. It also
derives owner-branch facts that group branch roots, roles, owner namespaces,
import roots, local dependencies, and child edges together. Package-level owner
dependency edges are exposed separately, so policy packs and agent snapshots do
not have to infer project structure from a massive file list. Child edges retain
their native relation kind: ordinary `mod`, explicit `#[path] mod`, or literal
`include!`.

Custom project source roots configured through `RustHarnessConfig` are treated
as source ownership roots by project, modularity, and agent policy packs.
Within those roots, `lib.rs` is treated as a crate facade: it should declare
external modules and re-export public API, while macro implementations and other
owned logic live in leaf modules. Parser-native facade exceptions cover
crate-root boundary forms seen in mature Rust crates: `extern crate`,
cfg-gated `compile_error!`, and macro invocations whose token body parses as
facade-only module declarations or re-exports. `mod.rs` has the same
special-file treatment at module boundaries: it should expose declarations and
re-exports, not own implementation bodies.
Binary entrypoints have the matching adapter contract: `src/main.rs` and
`src/bin` files should contain imports and `fn main`, while CLI parsing and
execution logic live in owned modules.
Root `build.rs` has the same thin-entrypoint contract for Cargo build-script
logic: keep imports and `fn main` there, and move larger build behavior behind a
build dependency.

The path-clarity surface is also project-scoped. Modularity policy consumes
native Rust `use` tree facts from the parser to reject `super::super` owner
escapes and all glob imports, including `use super::*`. `super::super` repairs
should use stable `crate::...` owner/facade imports; leaf implementation targets
should be exposed through their owner facade first. Those facts include
inline-module and `#[cfg(test)]` context, and import clarity policy runs over
both `src/` and conventional `tests/` roots. Agent advice reports repeated
namespace segments across the default package surface, including test helpers
and ordinary Rust file stems.

Reasoning-tree reachability uses the same parser boundary: `mod` declarations,
`#[path]` attributes, and literal `include!` source shards are resolved into
module-tree facts under `src/parser/` before `RUST-MOD-R007` and
`RUST-MOD-R009` render findings.

## Self-Apply Contract

The package is also self-hosted by its own default policy. The library target
mounts `rust_project_harness_cargo_test_gate!` from `src/self_policy.rs`. That
source-backed gate covers unfiltered `cargo test --lib` and ordinary
`cargo test` runs while keeping policy changes subject to the same gate
downstream projects consume. Downstream packages that need filter-proof
enforcement can instead mount the build-time gate from root `build.rs`; a
complete build gate satisfies the same project-gate contract.
Cargo-test embedding is intentionally stricter than the raw library runner:
`rust.agent_policy` findings remain `Info`, but the default cargo-test gate
fails on compact agent advice so the next repair agent can see and enrich the
project structure instead of losing the message inside passing test output.
Projects can clear that notification by fixing the structure, by configuring
the relevant rule surface, or by using an explicit
`advice = allow, config = { ... }` waiver.

New source-backed test modules should stay under `tests/unit` and be mounted
with `#[path]`. Harness-enabled library crates should mount either
`rust_project_harness_cargo_test_gate!(config = ...)` from a `#[cfg(test)]`
source module or a complete build-time gate from root `build.rs`; otherwise
they will be reported by `RUST-PROJ-R009`. Root test targets alone do not run
under `cargo test --lib`.
Root Cargo test targets are thin aggregates: they should mount external suite
modules only, while test bodies and helpers belong in suite files under
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
