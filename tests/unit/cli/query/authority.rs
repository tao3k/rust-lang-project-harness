use std::ffi::OsString;

#[test]
fn exact_query_extracts_typed_projection_authority_before_search_option_parsing() {
    let parser_digest =
        "blake3-256:1111111111111111111111111111111111111111111111111111111111111111";
    let query_pack_digest =
        "blake3-256:2222222222222222222222222222222222222222222222222222222222222222";
    let command = super::parse_query(
        [
            "--from-hook",
            "item-skeleton",
            "--selector",
            "rust://src/lib.rs#item/function/run",
            "--source-snapshot-envelope",
            "snapshot.json",
            "--code",
            "--json",
            "--asp-provider-id",
            "rs-harness",
            "--asp-parser-identity-digest",
            parser_digest,
            "--asp-query-pack-digest",
            query_pack_digest,
        ]
        .into_iter()
        .map(OsString::from),
    )
    .expect("parse typed exact projection query");

    let super::QueryCommand::ExactSource(options) = command else {
        panic!("typed exact projection must route to ExactSource");
    };
    assert_eq!(options.provider_id.as_deref(), Some("rs-harness"));
    assert_eq!(
        options.parser_identity_digest.as_deref(),
        Some(parser_digest)
    );
    assert_eq!(
        options.query_pack_digest.as_deref(),
        Some(query_pack_digest)
    );
    assert_eq!(
        options.source_snapshot_envelope.as_deref(),
        Some(std::path::Path::new("snapshot.json"))
    );
    assert!(options.code);
    assert!(options.json);
}

#[test]
fn typed_projection_authority_rejects_non_exact_query_routes() {
    let result = super::parse_query(
        ["--term", "run", "--asp-provider-id", "rs-harness"]
            .into_iter()
            .map(OsString::from),
    );
    let Err(error) = result else {
        panic!("typed authority must not leak into search routing");
    };
    assert!(error.contains("requires an exact --selector"), "{error}");
}
