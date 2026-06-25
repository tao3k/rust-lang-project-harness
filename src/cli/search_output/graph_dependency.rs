use std::collections::BTreeMap;

pub(super) fn render_dependency_graph(
    rendered: &str,
    fields: &BTreeMap<String, String>,
    limit: usize,
) -> String {
    let query = fields.get("q").map(String::as_str).unwrap_or("-");
    let algorithm = synthesis_field(rendered, "algorithm").unwrap_or("seed-frontier");
    let mut out = format!("[search-dependency] q={query} alg={algorithm}\n");
    out.push_str("legend: ID=kind:role(value)!next; edge SRC>{DST:rel}; frontier ID.next\n");
    out.push_str("aliases: graph:{G=search,D=dependency,I=import,C=crate,T=test}\n");
    let mut nodes = vec![DepNode::new("D", "dependency", "pkg", query, "dependency")];
    nodes.extend(dep_nodes(
        "T",
        "test",
        "path",
        dependency_test_seed_values(rendered),
        "tests",
        1,
        0,
    ));
    nodes.extend(dep_nodes(
        "O",
        "owner",
        "path",
        owner_values(rendered),
        "owner",
        1,
        0,
    ));
    nodes.extend(dep_nodes(
        "D",
        "dependency",
        "pkg",
        seed_values(rendered, "deps"),
        "deps",
        limit.saturating_sub(nodes.len()),
        1,
    ));
    nodes.extend(dep_nodes(
        "I",
        "import",
        "use",
        seed_values(rendered, "import"),
        "import",
        limit.saturating_sub(nodes.len()),
        0,
    ));
    let item_nodes = dep_nodes(
        "I",
        "item",
        "symbol",
        item_symbol_values(rendered),
        "syntax",
        limit.saturating_sub(nodes.len()).max(1),
        1,
    );
    nodes.extend(dep_nodes(
        "C",
        "crate",
        "source",
        seed_values(rendered, "crate-source"),
        "crate-source",
        limit.saturating_sub(nodes.len()),
        0,
    ));
    let mut rendered_nodes = nodes.iter().map(DepNode::render).collect::<Vec<_>>();
    rendered_nodes.extend(item_nodes.iter().map(DepNode::render));
    out.push_str(&rendered_nodes.join(";"));
    out.push('\n');
    out.push_str("G>{");
    out.push_str(
        &nodes
            .iter()
            .chain(item_nodes.iter())
            .map(|node| format!("{}:uses", node.id))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push_str("}\n");
    out.push_str("rank=");
    out.push_str(
        &nodes
            .iter()
            .take(limit)
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push_str(" frontier=");
    out.push_str(
        &nodes
            .iter()
            .take(limit)
            .map(|node| format!("{}.{}", node.id, node.next))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push('\n');
    if limit > 3 {
        append_passthrough_lines(&mut out, rendered, &["|next ", "avoid="]);
    }
    out
}

fn dependency_test_seed_values(rendered: &str) -> Vec<String> {
    if rendered.lines().any(|line| line == "|seed tests") {
        vec![".".to_string()]
    } else {
        test_values(rendered)
    }
}

fn item_symbol_values(rendered: &str) -> Vec<String> {
    line_second_tokens(rendered, "|item ")
}

fn dep_nodes(
    prefix: &str,
    kind: &str,
    value_kind: &str,
    values: Vec<String>,
    next: &str,
    limit: usize,
    start_index: usize,
) -> Vec<DepNode> {
    values
        .into_iter()
        .filter(|value| !value.is_empty() && value != "-")
        .take(limit)
        .enumerate()
        .map(|(index, value)| {
            DepNode::new(
                dep_node_id(prefix, start_index + index),
                kind,
                value_kind,
                value,
                next,
            )
        })
        .collect()
}

fn dep_node_id(prefix: &str, index: usize) -> String {
    if index == 0 {
        prefix.to_string()
    } else {
        format!("{prefix}{}", index + 1)
    }
}

struct DepNode {
    id: String,
    kind: String,
    value_kind: String,
    value: String,
    next: String,
}

impl DepNode {
    fn new(
        id: impl Into<String>,
        kind: impl Into<String>,
        value_kind: impl Into<String>,
        value: impl Into<String>,
        next: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            value_kind: value_kind.into(),
            value: value.into(),
            next: next.into(),
        }
    }

    fn render(&self) -> String {
        format!(
            "{}={}:{}({})!{}",
            self.id, self.kind, self.value_kind, self.value, self.next
        )
    }
}

fn seed_values(rendered: &str, seed_key: &str) -> Vec<String> {
    let prefix = format!("|seed {seed_key}:");
    rendered
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix))
        .flat_map(split_csv)
        .collect()
}

fn owner_values(rendered: &str) -> Vec<String> {
    let mut values = seed_values(rendered, "owner");
    if values.is_empty() {
        values = line_second_tokens(rendered, "|owner ");
    }
    if values.is_empty() {
        values = next_values(rendered, "owner:");
    }
    if values.is_empty()
        && let Some(seeds) = synthesis_field(rendered, "seeds")
    {
        values = seeds
            .split(',')
            .filter_map(|seed| seed.strip_prefix("owner:"))
            .map(str::to_string)
            .collect();
    }
    values
}

fn test_values(rendered: &str) -> Vec<String> {
    let mut values = seed_values(rendered, "tests");
    if values.is_empty() {
        values = line_second_tokens(rendered, "|test ");
    }
    if values.is_empty() {
        values = next_values(rendered, "tests:");
    }
    if values.is_empty()
        && let Some(seeds) = synthesis_field(rendered, "seeds")
    {
        values = seeds
            .split(',')
            .filter_map(|seed| seed.strip_prefix("tests:"))
            .map(str::to_string)
            .collect();
    }
    values
}

fn line_second_tokens(rendered: &str, prefix: &str) -> Vec<String> {
    rendered
        .lines()
        .filter_map(|line| line.strip_prefix(prefix))
        .filter_map(|rest| rest.split_whitespace().next())
        .flat_map(split_csv)
        .collect()
}

fn next_values(rendered: &str, value_prefix: &str) -> Vec<String> {
    rendered
        .lines()
        .filter_map(|line| line.strip_prefix("|next "))
        .flat_map(|rest| rest.split(','))
        .filter_map(|part| part.trim().strip_prefix(value_prefix))
        .map(str::to_string)
        .collect()
}

fn synthesis_field<'a>(rendered: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    rendered
        .lines()
        .filter_map(|line| line.strip_prefix("|synthesis "))
        .flat_map(str::split_whitespace)
        .find_map(|part| part.strip_prefix(&prefix))
}

fn append_passthrough_lines(out: &mut String, rendered: &str, prefixes: &[&str]) {
    for line in rendered.lines() {
        if prefixes.iter().any(|prefix| line.starts_with(prefix)) {
            out.push_str(line);
            out.push('\n');
        }
    }
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "-")
        .map(str::to_string)
        .collect()
}
