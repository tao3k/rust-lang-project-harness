# xiuxian-harness-rust-lang-project

`xiuxian-harness-rust-lang-project` is a project-level Rust language harness
library. It is a standalone extraction of the useful Rust project-governance
surface from `xiuxian-testing`, shaped like the Python project harness:
library-first APIs, deterministic rule catalogs, compact rendered diagnostics,
and non-blocking `AGENT-*` advice for repair-oriented agents.

It also ships a small CLI for local and CI policy runs. Compact text is the
default output; pass `--json` when a structured `RustHarnessReport` payload is
needed.

Project-root runners execute the full policy surface. By default they cover Rust
code under the crate's `src/`, `tests/`, `examples/`, and `benches/` roots, plus
root `build.rs` when it exists. Explicit-path runners are focused syntax probes
because they do not have a project scope.

## Self-Apply Policy

This crate applies the default project harness to itself. `src/lib.rs` mounts a
source-backed gate from `tests/unit/lib_policy.rs`, and `tests/unit_test.rs`
mounts the same default gate for the Cargo test target. That keeps the harness
rules honest: policy changes must pass through the package's own rule packs
before downstream projects inherit them.

Default assertions treat `Warning` and `Error` findings as blocking. `Info`
findings, including all `AGENT-*` advice, stay visible in compact rendered
diagnostics without failing the gate.

## Quick Use

```rust
use std::path::Path;

use xiuxian_harness_rust_lang_project::assert_rust_project_harness_clean;

#[test]
fn rust_project_harness_gate() {
    assert_rust_project_harness_clean(Path::new(env!("CARGO_MANIFEST_DIR")));
}
```

For a compact repair surface without panicking:

```rust
use std::path::Path;

use xiuxian_harness_rust_lang_project::{
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
```

## Current Rule Packs

Use `rust_rule_pack_descriptors()` for stable pack-level metadata. Default
project execution runs these packs in descriptor order:

- `rust.syntax`: blocks files that cannot be parsed by `syn`
- `rust.project_policy`: checks test layout, explicit test mounts, gate coverage, and thin root test targets
- `rust.modularity`: checks `lib.rs`/`mod.rs` facades, thin binary/build entrypoints, and source-shape drift
- `rust.agent_policy`: emits `AGENT-R001..R005` non-blocking advice for LLM repair

Rendered diagnostics are intentionally compact: rule id, source location,
highlighted source line when available, one short source pointer, `Help:`, and
`Contract:`. Structured consumers should keep using the serializable
`RustHarnessReport` shape through `render_rust_project_harness_json()`.

## Docs

Detailed package material lives under [`docs/`](docs/index.md).
