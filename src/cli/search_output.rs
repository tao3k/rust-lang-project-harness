//! Compact output controls shared by CLI search views.

use std::collections::{BTreeMap, BTreeSet};

pub(super) struct SearchOutputControls<'a> {
    pub(super) depth: Option<usize>,
    pub(super) output_view: Option<&'a str>,
    pub(super) seeds: Option<usize>,
}

pub(super) fn apply_search_output_controls(
    controls: SearchOutputControls<'_>,
    rendered: &str,
) -> String {
    if controls.depth == Some(0) {
        return render_header_only(rendered);
    }
    if controls.output_view == Some("seeds") {
        return render_search_seed_view(rendered, controls.seeds);
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

fn render_search_seed_view(rendered: &str, seed_limit: Option<usize>) -> String {
    const DEFAULT_SEED_LIMIT: usize = 8;

    let SearchSeeds {
        headers,
        mut seeds,
        notes,
    } = collect_search_seeds(rendered);
    seeds.sort_by(|left, right| {
        seed_priority(left)
            .cmp(&seed_priority(right))
            .then_with(|| seed_secondary_rank(left).cmp(&seed_secondary_rank(right)))
            .then_with(|| left.len().cmp(&right.len()))
            .then_with(|| left.cmp(right))
    });
    let seed_count = seeds.len();
    let seed_limit = seed_limit.unwrap_or(DEFAULT_SEED_LIMIT);
    let mut compact = String::new();
    for header in headers {
        compact.push_str(&header);
        compact.push('\n');
    }
    for seed in bounded_seeds(seeds, seed_limit) {
        compact.push_str("|seed ");
        compact.push_str(&seed);
        compact.push('\n');
    }
    if seed_count > seed_limit {
        compact.push_str(&format!(
            "|note seeds_truncated={} limit={seed_limit}\n",
            seed_count - seed_limit
        ));
    }
    for note in notes.into_iter().take(1) {
        compact.push_str(&note);
        compact.push('\n');
    }
    compact
}

fn seed_priority(seed: &str) -> usize {
    if seed.starts_with("dependency:") || seed.starts_with("deps:") || seed.starts_with("import:") {
        0
    } else if seed.starts_with("feature:") || seed.starts_with("features:") {
        1
    } else if seed.starts_with("cfg:") {
        2
    } else if seed.starts_with("owner:") {
        3
    } else if seed == "tests" || seed.starts_with("tests:") {
        4
    } else if seed.starts_with("docs:") || seed.starts_with("docs-use:") {
        5
    } else if seed.starts_with("text:") {
        6
    } else if seed.starts_with("symbol:") {
        7
    } else {
        8
    }
}

fn seed_secondary_rank(seed: &str) -> usize {
    let target = seed.rsplit(':').next().unwrap_or(seed);
    if matches!(target, "default" | "full" | "all") {
        3
    } else if seed.starts_with("owner:")
        && (target.ends_with("/lib.rs") || target.ends_with("/main.rs"))
    {
        2
    } else if target.chars().next().is_some_and(char::is_uppercase) || target.contains('-') {
        0
    } else if target.contains('_') {
        2
    } else {
        1
    }
}

fn bounded_seeds(seeds: Vec<String>, seed_limit: usize) -> Vec<String> {
    let first_pass = first_pass_seed_indices(&seeds, seed_limit);
    let first_pass_set = first_pass.iter().copied().collect::<BTreeSet<_>>();
    let remaining = seed_limit.saturating_sub(first_pass.len());
    first_pass
        .into_iter()
        .chain(
            (0..seeds.len())
                .filter(|index| !first_pass_set.contains(index))
                .take(remaining),
        )
        .map(|index| seeds[index].clone())
        .collect()
}

fn first_pass_seed_indices(seeds: &[String], seed_limit: usize) -> Vec<usize> {
    seeds
        .iter()
        .enumerate()
        .scan(
            BTreeMap::<usize, usize>::new(),
            |priority_counts, (index, seed)| {
                let priority = seed_priority(seed);
                let count = priority_counts.entry(priority).or_default();
                if *count >= first_pass_limit_for_priority(priority) {
                    return Some(None);
                }
                *count += 1;
                Some(Some(index))
            },
        )
        .flatten()
        .take(seed_limit)
        .collect()
}

fn first_pass_limit_for_priority(priority: usize) -> usize {
    if priority == 0 { 2 } else { 1 }
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
