//! Shared compact-code rendering from parser projection nodes.

pub(crate) fn compact_code_from_projection_nodes<'a, N: 'a>(
    nodes: impl IntoIterator<Item = &'a N>,
    mut node_parts: impl FnMut(&N) -> Option<(usize, String)>,
) -> String {
    nodes
        .into_iter()
        .filter_map(|node| node_parts(node))
        .fold(
            CompactCodeRenderState::default(),
            |state, (depth, label)| state.push_node(depth, label),
        )
        .finish()
}

#[derive(Default)]
struct CompactCodeRenderState {
    lines: Vec<String>,
    open_depths: Vec<usize>,
}

impl CompactCodeRenderState {
    fn push_node(mut self, depth: usize, label: String) -> Self {
        let label = label.trim();
        let label_consumed = self.close_projection_blocks(depth, Some(label));
        if !label_consumed && !label.is_empty() {
            self.lines
                .push(format!("{}{}", "    ".repeat(depth), label));
        }
        if label.ends_with('{') {
            self.open_depths.push(depth);
        }
        self
    }

    fn finish(mut self) -> String {
        self.close_projection_blocks(0, None);
        self.lines.join(
            "
",
        )
    }

    fn close_projection_blocks(&mut self, next_depth: usize, next_label: Option<&str>) -> bool {
        while self
            .open_depths
            .last()
            .is_some_and(|open_depth| *open_depth >= next_depth)
        {
            let open_depth = self.open_depths.pop().expect("checked open depth");
            let indent = "    ".repeat(open_depth);
            if open_depth == next_depth
                && let Some(label) = next_label
                && label.starts_with("else")
            {
                self.lines.push(format!("{indent}}} {label}"));
                return true;
            }
            self.lines.push(format!("{indent}}}"));
        }
        false
    }
}
