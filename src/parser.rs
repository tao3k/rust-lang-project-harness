//! Internal Rust parsing helpers.

use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::Span;

use crate::{RustModuleReport, SourceLocation};

pub(crate) struct ParsedRustModule {
    pub report: RustModuleReport,
    pub source: String,
    pub syntax: Option<syn::File>,
    pub error_span: Option<Span>,
}

pub(crate) fn parse_rust_file(path: &Path) -> ParsedRustModule {
    let source = fs::read_to_string(path).unwrap_or_default();
    match syn::parse_file(&source) {
        Ok(syntax) => ParsedRustModule {
            report: RustModuleReport {
                path: path.to_path_buf(),
                is_valid: true,
                parse_error: None,
            },
            source,
            syntax: Some(syntax),
            error_span: None,
        },
        Err(error) => ParsedRustModule {
            report: RustModuleReport {
                path: path.to_path_buf(),
                is_valid: false,
                parse_error: Some(error.to_string()),
            },
            source,
            syntax: None,
            error_span: Some(error.span()),
        },
    }
}

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
