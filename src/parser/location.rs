//! Source location helpers derived from native parser spans.

use std::path::PathBuf;

use proc_macro2::Span;

use crate::SourceLocation;

pub(crate) fn source_line(source: &str, line: usize) -> Option<String> {
    source
        .lines()
        .nth(line.saturating_sub(1))
        .map(str::to_string)
}

pub(crate) fn span_location(path: Option<PathBuf>, span: Span) -> SourceLocation {
    let start = span.start();
    SourceLocation::new(path, start.line.max(1), start.column)
}

pub(crate) fn file_location(path: impl Into<PathBuf>) -> SourceLocation {
    SourceLocation::new(Some(path.into()), 1, 0)
}

pub(crate) fn path_line_location(path: impl Into<PathBuf>, line: usize) -> SourceLocation {
    SourceLocation::new(Some(path.into()), line.max(1), 0)
}
