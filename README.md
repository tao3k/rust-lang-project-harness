# rust-lang-project-harness

`rust-lang-project-harness` is a project-level Rust language harness
library. It is a standalone extraction of the useful Rust project-governance
surface from `xiuxian-testing`, shaped like the Python project harness:
library-first APIs, deterministic rule catalogs, compact rendered diagnostics,
and non-blocking `AGENT-*` advice for repair-oriented agents.

It also ships a small CLI for local and CI policy runs. Compact text is the
default output; pass `--json` when a structured `RustHarnessReport` payload is
needed.

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
```

## Current Rule Packs

Use `rust_rule_pack_descriptors()` for stable pack-level metadata. Default
project execution runs these packs in descriptor order:

- `rust.syntax`: blocks files that cannot be parsed by `syn`
- `rust.project_policy`: checks test layout, explicit test mounts, gate coverage, and thin root test targets
- `rust.modularity`: checks `lib.rs`/`mod.rs` facades, thin binary/build entrypoints, and source-shape drift
- `rust.agent_policy`: emits `AGENT-R001..R008` non-blocking advice for LLM repair

Rendered diagnostics are intentionally compact: rule id, source location,
highlighted source line when available, one short source pointer, `Help:`, and
`Contract:`. Structured consumers should keep using the serializable
`RustHarnessReport` shape through `render_rust_project_harness_json()`.

## Docs

Detailed package material lives under [`docs/`](docs/index.md).
