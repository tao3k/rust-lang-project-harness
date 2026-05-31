//! Compact search trace rendering for final-only packets.

use std::fmt::Write as _;

pub(super) struct SearchTraceOptions {
    pub(super) source: String,
    pub(super) query: Option<String>,
    pub(super) pipes: Vec<String>,
    pub(super) output_view: Option<String>,
}

pub(super) fn render_search_trace(options: &SearchTraceOptions, rendered: &str) -> String {
    let counts = SearchTraceCounts::from_rendered(rendered);
    let mut trace = format!(
        "[search-trace] source={} query={} pipes={} view={}\n",
        options.source,
        options.query.as_deref().unwrap_or("-"),
        if options.pipes.is_empty() {
            "-".to_string()
        } else {
            options.pipes.join(",")
        },
        options.output_view.as_deref().unwrap_or("graph")
    );
    append_stage_summary(&mut trace, &counts);
    trace
}

#[derive(Default)]
struct SearchTraceCounts {
    packages: usize,
    features: usize,
    deps: usize,
    owners: usize,
    items: usize,
    api: usize,
    cfg: usize,
    tests: usize,
    edges: usize,
    lines: usize,
}

impl SearchTraceCounts {
    fn from_rendered(rendered: &str) -> Self {
        let mut counts = Self::default();
        for line in rendered.lines() {
            counts.lines += 1;
            let Some(line) = line.strip_prefix('|') else {
                continue;
            };
            counts.record_tag(line.split_whitespace().next().unwrap_or_default());
        }
        counts
    }

    fn record_tag(&mut self, tag: &str) {
        match tag {
            "package" => self.packages += 1,
            "feature" => self.features += 1,
            "dep" => self.deps += 1,
            "owner" => self.owners += 1,
            "item" => self.items += 1,
            "api" => self.api += 1,
            "cfg" => self.cfg += 1,
            "test" => self.tests += 1,
            "edge" => self.edges += 1,
            _ => {}
        }
    }
}

fn append_stage_summary(trace: &mut String, counts: &SearchTraceCounts) {
    trace.push_str("|stage");
    append_nonzero_count(trace, "packages", counts.packages);
    append_nonzero_count(trace, "features", counts.features);
    append_nonzero_count(trace, "cargo", counts.deps);
    append_nonzero_count(trace, "owners", counts.owners);
    append_nonzero_count(trace, "items", counts.items);
    append_nonzero_count(trace, "api", counts.api);
    append_nonzero_count(trace, "cfg", counts.cfg);
    append_nonzero_count(trace, "tests", counts.tests);
    append_nonzero_count(trace, "edges", counts.edges);
    let _ = writeln!(trace, " final=true lines={}", counts.lines);
}

fn append_nonzero_count(trace: &mut String, key: &str, count: usize) {
    if count > 0 {
        let _ = write!(trace, " {key}={count}");
    }
}
