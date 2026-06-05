use serde_json::{Map, Value, json};

use super::semantic_search_json_fields::{parse_fields, parse_next_actions};

pub(super) fn graph_seed_fragment(line: &str) -> Option<String> {
    let (id, payload) = line.split_once('=')?;
    if id.is_empty() || !id.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return None;
    }
    let (typed_target, action) = payload.rsplit_once('!')?;
    let (kind, target) = typed_target.split_once(':')?;
    let seed_kind = match (kind, action) {
        ("owner", "owner") => "owner",
        ("test" | "tests", "tests") => "tests",
        ("feature", "cfg" | "feature") => "feature",
        ("finding", "finding" | "owner" | "tests") => "finding",
        _ => return None,
    };
    let target = target.trim().trim_start_matches('{').trim_end_matches('}');
    let target = target
        .strip_prefix("path(")
        .and_then(|target| target.strip_suffix(')'))
        .unwrap_or(target);
    if target.is_empty() {
        return None;
    }
    Some(format!("{seed_kind}:{target}"))
}

pub(super) fn push_synthesis(tokens: &[&str], search_synthesis: &mut Option<Value>) {
    let mut fields = parse_fields(tokens.iter().copied());
    let mut synthesis = Map::new();
    for key in [
        "algorithm",
        "scope",
        "summary",
        "ownerPath",
        "selectedOwners",
        "selectedEdges",
        "incomingOwners",
        "outgoingOwners",
    ] {
        if let Some(value) = fields.remove(key) {
            synthesis.insert(key.to_string(), value);
        }
    }
    for key in [
        "highImpactOwners",
        "frontierOwners",
        "findingOwners",
        "editFrontier",
        "testFrontier",
    ] {
        if let Some(value) = fields.remove(key) {
            synthesis.insert(key.to_string(), ensure_array_value(value));
        }
    }
    if let Some(value) = fields.remove("seeds") {
        synthesis.insert(
            "seeds".to_string(),
            Value::Array(synthesis_next_actions(value)),
        );
    }
    if let Some(value) = fields.remove("windowSet") {
        synthesis.insert(
            "windowSet".to_string(),
            Value::Array(synthesis_next_actions(value)),
        );
    }
    normalize_synthesis_list_fields(&mut fields);
    if !fields.is_empty() {
        synthesis.insert("fields".to_string(), Value::Object(fields));
    }
    if synthesis.contains_key("algorithm") && synthesis.contains_key("scope") {
        *search_synthesis = Some(Value::Object(synthesis));
    }
}

fn ensure_array_value(value: Value) -> Value {
    match value {
        array @ Value::Array(_) => array,
        other => Value::Array(vec![other]),
    }
}

fn synthesis_next_actions(value: Value) -> Vec<Value> {
    match value {
        Value::Array(values) => values
            .into_iter()
            .flat_map(|value| match value {
                Value::String(value) => parse_next_actions(value, None),
                Value::Object(_) => vec![value],
                _ => Vec::new(),
            })
            .collect(),
        Value::String(value) => parse_next_actions(value, None),
        Value::Object(_) => vec![value],
        _ => Vec::new(),
    }
}

fn normalize_synthesis_list_fields(fields: &mut Map<String, Value>) {
    for key in ["windowSet"] {
        let Some(value) = fields.get_mut(key) else {
            continue;
        };
        if !value.is_array() {
            *value = Value::Array(vec![value.take()]);
        }
    }
}

pub(super) fn query_set_search_synthesis(
    owners: &[Value],
    nodes: &[Value],
    seed_fragments: &[String],
) -> Option<Value> {
    let mut edit_frontier = owners
        .iter()
        .filter_map(|owner| path_value(owner).map(|path| Value::String(path.to_string())))
        .collect::<Vec<_>>();
    let mut test_frontier = nodes
        .iter()
        .filter_map(|node| path_value(node).map(|path| Value::String(path.to_string())))
        .collect::<Vec<_>>();

    for (kind, target) in seed_targets(seed_fragments) {
        match kind {
            "owner" => edit_frontier.push(Value::String(target.to_string())),
            "test" | "tests" => test_frontier.push(Value::String(target.to_string())),
            _ => {}
        }
    }

    if edit_frontier.is_empty() && test_frontier.is_empty() {
        return None;
    }

    let mut window_set = Vec::new();
    window_set.extend(edit_frontier.iter().filter_map(|path| {
        path.as_str()
            .map(|path| json!({ "kind": "owner", "target": path }))
    }));
    window_set.extend(test_frontier.iter().filter_map(|path| {
        path.as_str()
            .map(|path| json!({ "kind": "tests", "target": path }))
    }));
    let seeds = window_set.clone();

    Some(json!({
        "algorithm": "change-frontier-query-set",
        "scope": "query-set",
        "summary": "query-set-frontier",
        "editFrontier": edit_frontier,
        "testFrontier": test_frontier,
        "windowSet": window_set,
        "seeds": seeds,
    }))
}

pub(super) fn merge_seed_fragment_search_synthesis(
    search_synthesis: &mut Option<Value>,
    scope: &str,
    seed_fragments: &[String],
) {
    let Some(seed_synthesis) = seed_fragment_search_synthesis(scope, seed_fragments) else {
        return;
    };
    let Some(existing) = search_synthesis.as_mut() else {
        *search_synthesis = Some(seed_synthesis);
        return;
    };
    for key in ["seeds", "windowSet", "editFrontier", "testFrontier"] {
        if existing.get(key).is_none() {
            if let Some(value) = seed_synthesis.get(key) {
                existing[key] = value.clone();
            }
        }
    }
}

fn seed_fragment_search_synthesis(scope: &str, seed_fragments: &[String]) -> Option<Value> {
    let frontier = seed_targets(seed_fragments).fold(
        SeedFragmentFrontier::default(),
        |mut frontier, (kind, target)| {
            frontier.push(kind, target);
            frontier
        },
    );
    if frontier.seeds.is_empty() {
        return None;
    }
    Some(json!({
        "algorithm": "seed-frontier",
        "scope": scope,
        "summary": "seed-frontier",
        "editFrontier": frontier.edit_frontier,
        "testFrontier": frontier.test_frontier,
        "windowSet": frontier.seeds,
        "seeds": frontier.seeds,
    }))
}

#[derive(Default)]
struct SeedFragmentFrontier {
    edit_frontier: Vec<Value>,
    test_frontier: Vec<Value>,
    seeds: Vec<Value>,
}

impl SeedFragmentFrontier {
    fn push(&mut self, kind: &str, target: &str) {
        let action_kind = match kind {
            "test" => "tests",
            other => other,
        };
        let action = json!({ "kind": action_kind, "target": target });
        self.seeds.push(action);
        match action_kind {
            "owner" => self.edit_frontier.push(Value::String(target.to_string())),
            "tests" => self.test_frontier.push(Value::String(target.to_string())),
            _ => {}
        }
    }
}

fn path_value(value: &Value) -> Option<&str> {
    value.get("path").and_then(Value::as_str)
}

fn seed_targets(seed_fragments: &[String]) -> impl Iterator<Item = (&str, &str)> {
    seed_fragments.iter().flat_map(|fragment| {
        let Some((kind, targets)) = fragment.split_once(':') else {
            return Vec::new().into_iter();
        };
        targets
            .split(',')
            .map(str::trim)
            .filter(|target| !target.is_empty())
            .map(move |target| (kind, target))
            .collect::<Vec<_>>()
            .into_iter()
    })
}
