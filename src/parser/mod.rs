//! Internal parser substrate for Rust source and Cargo project facts.

mod cargo_manifest;
mod cargo_test_targets;
mod location;
mod module_tree;
pub(crate) mod native_syntax;
mod parsed_module;
mod path_resolution;
mod reasoning_tree;
mod source_metrics;
mod source_path;

pub(crate) use cargo_manifest::{CargoManifestFacts, parse_cargo_manifest};
pub(crate) use cargo_test_targets::parse_cargo_test_targets;
pub(crate) use location::{file_location, path_line_location, source_line, span_location};
pub(crate) use module_tree::RustModuleChildEdge;
#[cfg(test)]
pub(crate) use module_tree::RustModuleChildEdgeKind;
pub(crate) use native_syntax::{RustNativeSyntaxFacts, RustTopLevelItemSyntax};
pub(crate) use parsed_module::{ParsedRustModule, parse_rust_file};
pub(crate) use reasoning_tree::{
    RustReasoningModuleFacts, RustReasoningTreeFacts, rust_reasoning_tree_facts,
};
pub(crate) use source_metrics::RustSourceMetrics;
pub(crate) use source_path::{RustSourcePathFacts, rust_source_path_facts};
