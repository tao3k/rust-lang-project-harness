# Rust Tree-sitter Query Corpus

This corpus fixes the ASP syntax ABI capture contract for Rust query catalogs.
It is stored beside the Rust provider catalogs, but validation, query
compilation, cache keys, and replay semantics are owned by the main ASP
workspace. The corpus is inspired by upstream `tree-sitter-rust/test/corpus`,
but it does not duplicate upstream grammar tests. Each case cites the upstream
corpus file that establishes the grammar shape, then records the finer ASP
capture expectations for `queries/*.scm`.

`grammar-profile.json` pins the ASP workspace git revision that validated this
query corpus contract. The validator checks that provenance is present; use
`--check-current-asp-revision` only when intentionally auditing a local checkout
against the pinned revision.

Cases may include `fixture-case:` metadata pointing at an existing
parser-compact real-library case. The validator resolves that manifest's
`fixtureRoot` and `rawSourcePath` so query capture text is checked against the
large-library fixture without copying the source into this corpus.

The corpus is a development and CI asset. Runtime providers continue to embed
catalog sources into the binary and do not require provider package source files.
