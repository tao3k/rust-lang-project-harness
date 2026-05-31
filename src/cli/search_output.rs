//! Compact output controls shared by CLI search views.

use std::collections::BTreeSet;

pub(super) struct SearchOutputControls<'a> {
    pub(super) depth: Option<usize>,
    pub(super) output_view: Option<&'a str>,
}

pub(super) fn apply_search_output_controls(
    controls: SearchOutputControls<'_>,
    rendered: &str,
) -> String {
    if controls.depth == Some(0) {
        return render_header_only(rendered);
    }
    if controls.output_view == Some("seeds") {
        return render_search_seed_view(rendered);
    }
    rendered.to_string()
}

fn render_header_only(rendered: &str) -> String {
    let mut header_only = rendered
        .lines()
        .filter(|line| !line.starts_with('|'))
        .collect::<Vec<_>>()
        .join("\n");
    if !header_only.is_empty() {
        header_only.push('\n');
    }
    header_only
}

fn render_search_seed_view(rendered: &str) -> String {
    const SEED_LIMIT: usize = 32;

    let SearchSeeds {
        headers,
        seeds,
        notes,
    } = collect_search_seeds(rendered);
    let seed_count = seeds.len();
    let mut compact = String::new();
    for header in headers {
        compact.push_str(&header);
        compact.push('\n');
    }
    for seed in seeds.into_iter().take(SEED_LIMIT) {
        compact.push_str("|seed ");
        compact.push_str(&seed);
        compact.push('\n');
    }
    if seed_count > SEED_LIMIT {
        compact.push_str(&format!(
            "|note seeds_truncated={} limit={SEED_LIMIT}\n",
            seed_count - SEED_LIMIT
        ));
    }
    for note in notes.into_iter().take(1) {
        compact.push_str(&note);
        compact.push('\n');
    }
    compact
}

struct SearchSeeds {
    headers: Vec<String>,
    seeds: Vec<String>,
    notes: Vec<String>,
}

fn collect_search_seeds(rendered: &str) -> SearchSeeds {
    rendered
        .lines()
        .fold(SearchSeedAccumulator::default(), |mut accumulator, line| {
            accumulator.record_line(line);
            accumulator
        })
        .finish()
}

#[derive(Default)]
struct SearchSeedAccumulator {
    headers: Vec<String>,
    seeds: Vec<String>,
    notes: Vec<String>,
    seen: BTreeSet<String>,
}

impl SearchSeedAccumulator {
    fn record_line(&mut self, line: &str) {
        if !line.starts_with('|') {
            self.headers.push(line.to_string());
            return;
        }
        if line.starts_with("|note ") {
            self.notes.push(line.to_string());
        }
        collect_next_seeds(line, &mut self.seen, &mut self.seeds);
    }

    fn finish(self) -> SearchSeeds {
        SearchSeeds {
            headers: self.headers,
            seeds: self.seeds,
            notes: self.notes,
        }
    }
}

fn collect_next_seeds(line: &str, seen: &mut BTreeSet<String>, seeds: &mut Vec<String>) {
    let rest = if let Some(next_actions) = line.strip_prefix("|next ") {
        next_actions
    } else {
        let Some((_, rest)) = line.split_once("next=") else {
            return;
        };
        rest
    };
    let next = rest.split_whitespace().next().unwrap_or(rest);
    for seed in next
        .split(',')
        .map(str::trim)
        .filter(|seed| !seed.is_empty())
    {
        if seen.insert(seed.to_string()) {
            seeds.push(seed.to_string());
        }
    }
}
