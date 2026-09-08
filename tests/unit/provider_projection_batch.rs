use serde_json::{Value, json};
use std::time::{Duration, Instant};

use super::handle_language_projection_batch_value;

fn request(owners: Value) -> Value {
    json!({
        "schemaId": "agent.semantic-protocols.provider-language-projection-batch-request",
        "schemaVersion": "1",
        "languageId": "rust",
        "providerId": "asp-rust",
        "workspaceIdentity": "workspace-test",
        "generationRootDigest": "blake3-256:generation",
        "parserIdentityDigest": "blake3-256:parser",
        "queryPackDigest": "blake3-256:query-pack",
        "baseGenerationRootDigest": null,
        "owners": owners,
        "auxiliaryOwners": [],
    })
}

#[test]
fn projection_batch_isolates_one_syntax_unavailable_owner() {
    let response = handle_language_projection_batch_value(&request(json!([
        {
            "ownerPath": "src/ready.rs",
            "sourceLeafDigest": "blake3-256:ready",
            "sourceEncoding": "utf8",
            "sourceText": "pub fn ready() -> usize { 1 }\n"
        },
        {
            "ownerPath": "src/unavailable.rs",
            "sourceLeafDigest": "blake3-256:unavailable",
            "sourceEncoding": "utf8",
            "sourceText": "pub fn unavailable( {\n"
        }
    ])))
    .expect("projection batch remains transport-ready");
    let response: Value = serde_json::from_slice(&response).expect("response JSON");
    let owners = response["owners"].as_array().expect("owners");

    assert_eq!(owners[0]["projectionState"], "ready");
    assert!(owners[0]["diagnostic"].is_null());
    assert!(
        !owners[0]["items"]
            .as_array()
            .expect("ready items")
            .is_empty()
    );
    assert_eq!(owners[1]["projectionState"], "syntax-unavailable");
    assert_eq!(
        owners[1]["diagnostic"]["reasonKind"],
        "source-syntax-unavailable"
    );
    assert!(
        owners[1]["items"]
            .as_array()
            .expect("unavailable items")
            .is_empty()
    );
    assert!(
        owners[1]["relations"]
            .as_array()
            .expect("unavailable relations")
            .is_empty()
    );
}

#[test]
fn representative_projection_batch_p99_stays_below_runtime_frame_budget() {
    const OWNER_COUNT: usize = 32;
    const CALLABLES_PER_OWNER: usize = 32;
    const SAMPLE_COUNT: usize = 16;
    let owners = (0..OWNER_COUNT)
        .map(|owner| {
            let mut source = String::new();
            for callable in 0..CALLABLES_PER_OWNER {
                source.push_str(&format!(
                    "pub fn owner_{owner:02}_callable_{callable:02}(value: usize) -> usize {{ value.wrapping_add({callable}) }}\n"
                ));
            }
            json!({
                "ownerPath": format!("src/owner_{owner:02}.rs"),
                "sourceLeafDigest": format!("blake3-256:owner-{owner:02}"),
                "sourceEncoding": "utf8",
                "sourceText": source,
            })
        })
        .collect::<Vec<_>>();
    let request = request(Value::Array(owners));
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let started = Instant::now();
        let response = handle_language_projection_batch_value(&request)
            .expect("representative projection batch");
        samples.push(started.elapsed());
        let response: Value = serde_json::from_slice(&response).expect("projection response JSON");
        assert_eq!(
            response["owners"]
                .as_array()
                .expect("projected owners")
                .len(),
            OWNER_COUNT
        );
    }
    samples.sort_unstable();
    let p50 = samples[(SAMPLE_COUNT - 1) * 50 / 100];
    let p95 = samples[(SAMPLE_COUNT - 1) * 95 / 100];
    let p99 = samples[(SAMPLE_COUNT - 1) * 99 / 100];
    eprintln!(
        "rust projection batch receipt: owners={OWNER_COUNT} callablesPerOwner={CALLABLES_PER_OWNER} samples={SAMPLE_COUNT} p50Micros={} p95Micros={} p99Micros={} frameDeadlineMillis=2000",
        p50.as_micros(),
        p95.as_micros(),
        p99.as_micros(),
    );
    assert!(p99 < Duration::from_millis(250));
}
