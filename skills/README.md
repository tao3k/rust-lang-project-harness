# rust-lang-project-harness Skills

This directory contains repo-local Agent skills for working on
`rust-lang-project-harness`.

The skills are intentionally small and harness-specific. They are not a vendored
copy of general Rust best-practice catalogs. They tell an Agent how to apply
this repository's parser-first policy, compact output contract, and verification
planning API.

## When To Load

- Load [`rust-harness-policy`](rust-harness-policy/SKILL.md) before changing
  `src/` or `tests/` Rust code in this repository.
- Load [`rust-verification-profile`](rust-verification-profile/SKILL.md) when
  editing verification configuration, receipts, waivers, or responsibility
  profile hints.
- Load [`rust-verification-performance`](rust-verification-performance/SKILL.md)
  when a task touches latency-sensitive, throughput-sensitive,
  allocation-sensitive, parser-loop, renderer-loop, async, or hot-path code.

## Reference Repositories Studied

These repositories were shallow-cloned under `.run/tmp` on 2026-05-01 and used
only as organization references:

- [`leonardomso/rust-skills`](https://github.com/leonardomso/rust-skills)
  (`89910e858533`): useful as a compact rule-catalog shape.
- [`actionbook/rust-skills`](https://github.com/actionbook/rust-skills)
  (`1f4becdcb88d`): useful as a skill-folder and routing shape.

Do not copy their rule text into this repository. This repository's skills must
remain original, compact, and tied to `rust-lang-project-harness` facts.
