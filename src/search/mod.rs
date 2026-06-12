//! Compact search protocol renderers for agent exploration.

mod api;
mod cargo;
pub(crate) mod compact;
mod context;
mod dependency;
mod format;
mod fzf_query;
pub(crate) mod guide;
mod hits;
mod ingest;
mod item_query;
mod limits;
mod namespace;
mod owner;
mod owner_view;
pub(crate) mod policy;
mod prime;
mod prime_support;
mod query;
mod recency;
mod scope;
#[cfg(feature = "cli")]
mod semantic_facts;
mod syntax_query;

pub use api::{
    RustSearchOptions, RustSearchViewRequest, render_rust_project_harness_search_view_with_config,
};
pub use ingest::render_rust_project_harness_search_ingest_with_config;
pub use prime::{
    render_rust_project_harness_search_prime, render_rust_project_harness_search_prime_with_config,
};
#[cfg(feature = "cli")]
pub use semantic_facts::render_rust_project_harness_search_semantic_facts_json;
