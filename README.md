# rust-lang-project-harness

`rust-lang-project-harness` is a project-level Rust language harness for
repair-oriented coding agents. It complements `rustc`, `rustfmt`, and Clippy by
turning native Rust syntax and Cargo manifest facts into compact
package/module/owner/dependency policy feedback.

The primary interface is intentionally agent-first: compact text by default,
JSON for structured tools, and focused search packets for orientation before
editing. The crate is library-first; the CLI is available behind the `cli`
feature.

## What It Does

- Builds parser-native project facts from Rust source and Cargo manifests.
- Runs deterministic rule packs for syntax, project policy, modularity, and
  agent repair advice.
- Provides a build-script `cargo check` gate for downstream crates.
- Exposes compact CLI/search/agent integration through `rs-harness`.
- Plans verification obligations for external skills without running benchmarks,
  stress tests, security scanners, or other runtime tools itself.

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

Run the local CLI when you need the same compact surface from a shell:

```shell
cargo run --features cli --bin rs-harness -- .
cargo run --features cli --bin rs-harness -- search prime --view seeds --seeds 8 .
cargo run --features cli --bin rs-harness -- check --changed .
cargo run --features cli --bin rs-harness -- agent install --client codex .
```

Global install helpers live in [`Justfile`](Justfile):

```shell
just install-bin-macos
just install-bin-linux
```

## Development

This crate self-applies the default project harness. Downstream crates should
prefer the build-time `cargo check` gate; this crate uses a self-apply path
because it cannot build-depend on itself.

Useful local checks:

```shell
cargo test --features cli
cargo clippy --features cli --all-targets -- -D warnings
rs-harness check --changed .
```

## Docs

Detailed package material lives under [`docs/`](docs/index.md):

- [Overview](docs/00_overview.md)
- [Harness Boundary](docs/01_core/101_harness_boundary.md)
- [Rule Catalog](docs/03_features/201_rule_catalog.md)
- [Runner Modes](docs/03_features/202_runner_modes.md)
- [CLI](docs/03_features/203_cli.md)
- [Verification Policy](docs/03_features/204_verification_policy.md)
- [Repo-Local Agent Skills](skills/README.md)
