//! Source-level metrics owned by the parser layer.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RustSourceMetrics {
    pub effective_code_lines: usize,
}

pub(crate) fn rust_source_metrics(source: &str) -> RustSourceMetrics {
    RustSourceMetrics {
        effective_code_lines: count_effective_code_lines(source),
    }
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
