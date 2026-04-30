# Development

## Format, test, lint

```shell
direnv exec . cargo fmt --all -- --check
direnv exec . cargo test
direnv exec . cargo clippy --all-targets --all-features -- -D warnings
direnv exec . git diff --check
```

## Self-applied policy

The crate runs its own project policy gate in both supported embedding modes:

- `src/lib.rs` mounts `tests/unit/lib_policy.rs` through
  `rust_project_harness_source_gate!`;
- `tests/unit_test.rs` mounts `rust_project_harness_gate!` directly.

When adding tests, keep behavior coverage under `tests/unit` and include it from
`tests/unit_test.rs` unless a new root target is intentionally part of the
policy surface.
Root Cargo test targets should stay as thin harness aggregates: gate macro plus
external module mounts only. Put test functions and helpers in suite files under
`tests/unit` or another documented suite directory.
Root-target module mounts should always be explicit `#[path = "suite/file.rs"]`
attributes rather than bare Rust `mod helper;` declarations.
Root `build.rs`, when present, is scanned by the project harness and should stay
a thin Cargo build-script entrypoint.

Intentional non-standard test roots or directories belong in
`tests/rust-project-harness-rules.toml`, and each entry must carry a non-empty
`explanation`.

## Policy closure

Default project assertions block on `Warning` and `Error`. `AGENT-*` rules stay
`Info`: rendered by default as repair advice, but non-blocking unless a caller
opts into stricter severity selection.

The `rust-project-harness` CLI follows the same contract. Compact text is the
default output for agent repair loops; `--json` emits the structured report for
tooling. CLI tests live under `tests/unit` and are mounted by the existing
`tests/unit_test.rs` target.

## Renderer snapshots

Compact text and JSON renderer contracts are locked by repo-local snapshot files
under `tests/unit/snapshots`. Update them only when the LLM-facing text shape or
structured JSON contract intentionally changes.
