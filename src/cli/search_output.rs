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
    if controls.output_view == Some("both") {
        return render_search_both_view(rendered);
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

fn render_search_both_view(rendered: &str) -> String {
    render_blocks(compact_package_blocks(parse_blocks(rendered)))
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
    let has_positive_header = headers
        .iter()
        .any(|header| header_has_positive_count(header));
    let mut compact = String::new();
    for header in compact_package_headers(headers) {
        compact.push_str(&header);
        compact.push('\n');
    }
    for seed in compact_seed_lines(bounded_seeds(seeds, seed_limit)) {
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
    let notes = if has_positive_header {
        notes
            .into_iter()
            .filter(|note| !note.contains("kind=not-found"))
            .collect::<Vec<_>>()
    } else {
        notes
    };
    for note in notes.into_iter().take(1) {
        compact.push_str(&note);
        compact.push('\n');
    }
    compact
}

struct SearchBlock {
    header: String,
    details: Vec<String>,
}

enum BlockEntry {
    Raw(SearchBlock),
    PackageGroup(PackageBlockGroup),
}

struct PackageBlockGroup {
    prefix: String,
    suffix: String,
    packages: Vec<String>,
    details: Vec<String>,
}

fn parse_blocks(rendered: &str) -> Vec<SearchBlock> {
    let mut blocks = Vec::<SearchBlock>::new();
    let mut current = None::<SearchBlock>;
    for line in rendered.lines() {
        if !line.starts_with('|') {
            if let Some(block) = current.take() {
                blocks.push(block);
            }
            current = Some(SearchBlock {
                header: line.to_string(),
                details: Vec::new(),
            });
        } else if let Some(block) = &mut current {
            block.details.push(line.to_string());
        } else {
            current = Some(SearchBlock {
                header: String::new(),
                details: vec![line.to_string()],
            });
        }
    }
    if let Some(block) = current {
        blocks.push(block);
    }
    blocks
}

fn compact_package_blocks(blocks: Vec<SearchBlock>) -> Vec<SearchBlock> {
    let mut entries = Vec::<BlockEntry>::new();
    for block in blocks {
        let Some(parts) = package_header_parts(&block.header) else {
            entries.push(BlockEntry::Raw(block));
            continue;
        };
        if let Some(BlockEntry::PackageGroup(group)) =
            entries.iter_mut().find(|entry| match entry {
                BlockEntry::PackageGroup(group) => {
                    group.prefix == parts.prefix
                        && group.suffix == parts.suffix
                        && group.details == block.details
                }
                BlockEntry::Raw(_) => false,
            })
        {
            group.packages.push(parts.package);
            continue;
        }
        entries.push(BlockEntry::PackageGroup(PackageBlockGroup {
            prefix: parts.prefix,
            suffix: parts.suffix,
            packages: vec![parts.package],
            details: block.details,
        }));
    }
    entries
        .into_iter()
        .map(|entry| match entry {
            BlockEntry::Raw(block) => block,
            BlockEntry::PackageGroup(group) => group.render(),
        })
        .collect()
}

impl PackageBlockGroup {
    fn render(self) -> SearchBlock {
        SearchBlock {
            header: format!(
                "{}pkg={}{}",
                self.prefix,
                self.packages.join(","),
                self.suffix
            ),
            details: self.details,
        }
    }
}

fn render_blocks(blocks: Vec<SearchBlock>) -> String {
    let mut rendered = String::new();
    for block in blocks {
        if !block.header.is_empty() {
            rendered.push_str(&block.header);
            rendered.push('\n');
        }
        for detail in block.details {
            rendered.push_str(&detail);
            rendered.push('\n');
        }
    }
    rendered
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
    if seed.starts_with("cfg:feature:") {
        0
    } else if seed == "cfg:test" || matches!(target, "default" | "full" | "all") {
        3
    } else if seed.starts_with("owner:tests/task_") {
        0
    } else if seed.starts_with("owner:tests/") {
        1
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

fn header_has_positive_count(header: &str) -> bool {
    header
        .split_whitespace()
        .filter_map(|field| field.split_once('='))
        .any(|(key, value)| {
            matches!(
                key,
                "api" | "calls" | "cfg" | "dep" | "defs" | "hit" | "item" | "own" | "tests"
            ) && value.parse::<usize>().is_ok_and(|count| count > 0)
        })
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

enum HeaderEntry {
    Raw(String),
    PackageGroup(PackageHeaderGroup),
}

struct PackageHeaderGroup {
    prefix: String,
    suffix: String,
    packages: Vec<String>,
}

struct PackageHeaderParts {
    prefix: String,
    package: String,
    suffix: String,
}

fn compact_package_headers(headers: Vec<String>) -> Vec<String> {
    let mut entries = Vec::<HeaderEntry>::new();
    for header in headers {
        let Some(parts) = package_header_parts(&header) else {
            entries.push(HeaderEntry::Raw(header));
            continue;
        };
        if let Some(HeaderEntry::PackageGroup(group)) = entries.iter_mut().find(|entry| match entry
        {
            HeaderEntry::PackageGroup(group) => {
                group.prefix == parts.prefix && group.suffix == parts.suffix
            }
            HeaderEntry::Raw(_) => false,
        }) {
            group.packages.push(parts.package);
            continue;
        }
        entries.push(HeaderEntry::PackageGroup(PackageHeaderGroup {
            prefix: parts.prefix,
            suffix: parts.suffix,
            packages: vec![parts.package],
        }));
    }
    entries
        .into_iter()
        .map(|entry| match entry {
            HeaderEntry::Raw(header) => header,
            HeaderEntry::PackageGroup(group) => group.render(),
        })
        .collect()
}

impl PackageHeaderGroup {
    fn render(self) -> String {
        format!(
            "{}pkg={}{}",
            self.prefix,
            self.packages.join(","),
            self.suffix
        )
    }
}

fn package_header_parts(line: &str) -> Option<PackageHeaderParts> {
    let mut cursor = 0;
    for field in line.split_whitespace() {
        let field_start = cursor + line[cursor..].find(field)?;
        let field_end = field_start + field.len();
        cursor = field_end;
        if let Some(package) = field.strip_prefix("pkg=") {
            return Some(PackageHeaderParts {
                prefix: line[..field_start].to_string(),
                package: package.to_string(),
                suffix: line[field_end..].to_string(),
            });
        }
    }
    None
}

enum SeedLine {
    Raw(String),
    Group { kind: String, targets: Vec<String> },
}

fn compact_seed_lines(seeds: Vec<String>) -> Vec<String> {
    let mut entries = Vec::<SeedLine>::new();
    for seed in seeds {
        let Some((kind, target)) = groupable_seed_parts(&seed) else {
            entries.push(SeedLine::Raw(seed));
            continue;
        };
        if let Some(SeedLine::Group { targets, .. }) = entries.iter_mut().find(|entry| {
            matches!(
                entry,
                SeedLine::Group {
                    kind: existing,
                    ..
                } if existing == kind
            )
        }) {
            targets.push(target.to_string());
            continue;
        }
        entries.push(SeedLine::Group {
            kind: kind.to_string(),
            targets: vec![target.to_string()],
        });
    }
    entries
        .into_iter()
        .map(|entry| match entry {
            SeedLine::Raw(seed) => seed,
            SeedLine::Group { kind, targets } => format!("{kind}:{}", targets.join(",")),
        })
        .collect()
}

fn groupable_seed_parts(seed: &str) -> Option<(&str, &str)> {
    let (kind, target) = seed.split_once(':')?;
    if !groupable_seed_kind(kind) || target.is_empty() || target.contains(',') {
        return None;
    }
    Some((kind, target))
}

fn groupable_seed_kind(kind: &str) -> bool {
    matches!(
        kind,
        "cfg"
            | "dependency"
            | "deps"
            | "docs"
            | "docs-use"
            | "feature"
            | "features"
            | "import"
            | "owner"
            | "package"
            | "prime"
            | "symbol"
            | "tests"
    )
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
    current_package: Option<String>,
}

impl SearchSeedAccumulator {
    fn record_line(&mut self, line: &str) {
        if !line.starts_with('|') {
            self.current_package = package_from_header(line);
            self.headers.push(line.to_string());
            return;
        }
        if line.starts_with("|note ") {
            self.notes.push(line.to_string());
        }
        if let Some(seed) = line.strip_prefix("|seed ") {
            record_seed(seed, &mut self.seen, &mut self.seeds);
            return;
        }
        collect_next_seeds(
            line,
            self.current_package.as_deref(),
            &mut self.seen,
            &mut self.seeds,
        );
    }

    fn finish(self) -> SearchSeeds {
        SearchSeeds {
            headers: self.headers,
            seeds: self.seeds,
            notes: self.notes,
        }
    }
}

fn record_seed(seed: &str, seen: &mut BTreeSet<String>, seeds: &mut Vec<String>) {
    let seed = seed.trim();
    if seed.is_empty() {
        return;
    }
    if seen.insert(seed.to_string()) {
        seeds.push(seed.to_string());
    }
}

fn collect_next_seeds(
    line: &str,
    package: Option<&str>,
    seen: &mut BTreeSet<String>,
    seeds: &mut Vec<String>,
) {
    let rest = if let Some(next_actions) = line.strip_prefix("|next ") {
        next_actions
    } else {
        let Some((_, rest)) = line.split_once("next=") else {
            return;
        };
        rest
    };
    let next = rest.split_whitespace().next().unwrap_or(rest);
    for seed in split_next_actions(next)
        .into_iter()
        .map(|seed| qualify_package_seed(&seed, package))
    {
        if seen.insert(seed.clone()) {
            seeds.push(seed);
        }
    }
}

fn split_next_actions(next: &str) -> Vec<String> {
    next.chars()
        .fold(NextActionSplit::default(), NextActionSplit::push)
        .finish()
}

#[derive(Default)]
struct NextActionSplit {
    actions: Vec<String>,
    current: String,
    brace_depth: usize,
}

impl NextActionSplit {
    fn push(mut self, character: char) -> Self {
        match character {
            ',' if self.brace_depth == 0 => self.flush_current(),
            '{' => {
                self.brace_depth += 1;
                self.current.push(character);
                self
            }
            '}' => {
                self.brace_depth = self.brace_depth.saturating_sub(1);
                self.current.push(character);
                self
            }
            _ => {
                self.current.push(character);
                self
            }
        }
    }

    fn finish(mut self) -> Vec<String> {
        self = self.flush_current();
        self.actions
    }

    fn flush_current(mut self) -> Self {
        let action = self.current.trim();
        if !action.is_empty() {
            self.actions.push(action.to_string());
        }
        self.current.clear();
        self
    }
}

fn package_from_header(line: &str) -> Option<String> {
    line.split_whitespace()
        .find_map(|field| {
            field
                .strip_prefix("pkg=")
                .or_else(|| field.strip_prefix("package="))
        })
        .filter(|package| !matches!(*package, "." | "-"))
        .map(ToOwned::to_owned)
}

fn qualify_package_seed(seed: &str, package: Option<&str>) -> String {
    let Some(package) = package else {
        return seed.to_string();
    };
    for prefix in ["owner:", "tests:", "ingest:", "text:"] {
        if let Some(target) = seed.strip_prefix(prefix)
            && package_seed_target_needs_prefix(target, package)
        {
            return format!("{prefix}{package}/{target}");
        }
    }
    seed.to_string()
}

fn package_seed_target_needs_prefix(target: &str, package: &str) -> bool {
    !target.starts_with('/')
        && !target.starts_with('<')
        && seed_target_looks_like_path(target)
        && target != package
        && !target
            .strip_prefix(package)
            .is_some_and(|rest| rest.starts_with('/'))
        && !target.contains("://")
}

fn seed_target_looks_like_path(target: &str) -> bool {
    target.contains('/') || target.ends_with(".rs")
}
