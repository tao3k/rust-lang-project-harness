//! Source-level metrics owned by the parser layer.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RustSourceMetrics {
    pub source_lines: usize,
    pub effective_code_lines: usize,
}

pub(crate) fn rust_source_metrics(source: &str) -> RustSourceMetrics {
    RustSourceMetrics {
        source_lines: count_source_lines(source),
        effective_code_lines: count_effective_code_lines(source),
    }
}

fn count_source_lines(source: &str) -> usize {
    source.lines().count()
}

fn count_effective_code_lines(source: &str) -> usize {
    source
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with("//")
                && !line.starts_with("/*")
                && !line.starts_with('*')
                && !line.starts_with("*/")
                && !line.starts_with("#[")
                && !line.starts_with("#![")
        })
        .count()
}
