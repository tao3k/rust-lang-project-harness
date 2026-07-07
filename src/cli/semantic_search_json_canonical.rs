use serde_json::{Map, Value};

pub(super) fn canonical_query_set_terms(
    _view: &str,
    _query: Option<&str>,
    query_set: &[String],
    _header_fields: &Map<String, Value>,
) -> Vec<String> {
    if !query_set.is_empty() {
        return query_set.to_vec();
    }
    Vec::new()
}

pub(super) fn canonical_owner_path(path: &str, owner: Option<&str>, query: Option<&str>) -> String {
    for owner in [owner, query].into_iter().flatten() {
        if owner == path || owner.ends_with(&format!("/{path}")) {
            return owner.to_string();
        }
    }
    path.to_string()
}

pub(super) fn canonicalize_read_field(fields: &mut Map<String, Value>, owner_path: &str) {
    let Some(read) = fields.get("read").and_then(Value::as_str) else {
        return;
    };
    let Some(canonical) = canonical_read_locator(read, owner_path) else {
        return;
    };
    fields.insert("read".to_string(), Value::String(canonical));
}

fn canonical_read_locator(read: &str, owner_path: &str) -> Option<String> {
    let (path_and_start, end) = read.rsplit_once(':')?;
    let (path, start) = path_and_start.rsplit_once(':')?;
    if owner_path == path || owner_path.ends_with(&format!("/{path}")) {
        Some(format!("{owner_path}:{start}:{end}"))
    } else {
        None
    }
}
