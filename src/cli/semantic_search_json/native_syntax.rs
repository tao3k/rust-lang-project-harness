use std::collections::BTreeSet;

use serde_json::{Value, json};

pub(super) fn append_native_syntax_relation_edges(
    native_syntax_facts: &[Value],
    edges: &mut Vec<Value>,
) {
    let mut seen = edges
        .iter()
        .filter_map(|edge| {
            Some((
                edge["from"].as_str()?.to_string(),
                edge["kind"].as_str()?.to_string(),
                edge["to"].as_str()?.to_string(),
            ))
        })
        .collect::<BTreeSet<_>>();
    for fact in native_syntax_facts {
        let Some(source_id) = fact["id"].as_str() else {
            continue;
        };
        let Some(relations) = fact["relations"].as_array() else {
            continue;
        };
        for relation in relations {
            let (Some(kind), Some(target)) =
                (relation["kind"].as_str(), relation["target"].as_str())
            else {
                continue;
            };
            if !seen.insert((source_id.to_string(), kind.to_string(), target.to_string())) {
                continue;
            }
            let mut edge = json!({
                "from": source_id,
                "kind": kind,
                "to": target,
                "fields": {
                    "source": "nativeSyntaxFacts.relations",
                    "sourceAuthority": fact["source"].as_str().unwrap_or("native-parser"),
                }
            });
            if let Some(location) = fact.get("location") {
                edge["location"] = location.clone();
            }
            edges.push(edge);
        }
    }
}
