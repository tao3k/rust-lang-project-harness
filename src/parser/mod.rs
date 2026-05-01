//! Internal parser substrate for Rust source and Cargo project facts.

mod cargo_manifest;
mod location;
pub(crate) mod native_syntax;
mod parsed_module;
mod source_metrics;

pub(crate) use cargo_manifest::{CargoManifestFacts, parse_cargo_manifest};
pub(crate) use location::{file_location, path_line_location, source_line, span_location};
pub(crate) use native_syntax::{RustNativeSyntaxFacts, RustTopLevelItemSyntax};
pub(crate) use parsed_module::{ParsedRustModule, parse_rust_file};
pub(crate) use source_metrics::RustSourceMetrics;
