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
    append_source_stage(&mut trace, &options.source, &counts);
    for pipe in &options.pipes {
        append_pipe_stage(&mut trace, pipe, &counts);
    }
    let _ = writeln!(trace, "|stage output final=true lines={}", counts.lines);
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

fn append_source_stage(trace: &mut String, source: &str, counts: &SearchTraceCounts) {
    match source {
        "workspace" => {
            let _ = writeln!(trace, "|stage workspace packages={}", counts.packages);
        }
        "dependency" => {
            let _ = writeln!(
                trace,
                "|stage dependency cargo={} owners={}",
                counts.deps, counts.owners
            );
        }
        "deps" => {
            let _ = writeln!(
                trace,
                "|stage deps cargo={} owners={} api={}",
                counts.deps, counts.owners, counts.api
            );
        }
        "features" => {
            let _ = writeln!(
                trace,
                "|stage features features={} deps={}",
                counts.features, counts.deps
            );
        }
        "cfg" => {
            let _ = writeln!(
                trace,
                "|stage cfg cfg={} owners={}",
                counts.cfg, counts.owners
            );
        }
        "owner" => {
            let _ = writeln!(
                trace,
                "|stage owner owners={} items={}",
                counts.owners, counts.items
            );
        }
        other => {
            let _ = writeln!(trace, "|stage {other} lines={}", counts.lines);
        }
    }
}

fn append_pipe_stage(trace: &mut String, pipe: &str, counts: &SearchTraceCounts) {
    match pipe {
        "owners" => {
            let _ = writeln!(trace, "|stage owners owners={}", counts.owners);
        }
        "items" => {
            let _ = writeln!(
                trace,
                "|stage items owners={} items={}",
                counts.owners, counts.items
            );
        }
        "docs" | "docs-use" => {
            let _ = writeln!(trace, "|stage docs docs={}", counts.api);
        }
        "tests" => {
            let _ = writeln!(trace, "|stage tests tests={}", counts.tests);
        }
        "public-api" => {
            let _ = writeln!(trace, "|stage public-api api={}", counts.api);
        }
        "cfg" => {
            let _ = writeln!(trace, "|stage cfg cfg={}", counts.cfg);
        }
        "features" => {
            let _ = writeln!(trace, "|stage features features={}", counts.features);
        }
        "dependents" => {
            let _ = writeln!(trace, "|stage dependents edges={}", counts.edges);
        }
        other => {
            let _ = writeln!(trace, "|stage {other} lines={}", counts.lines);
        }
    }
}
