//! Compact search protocol renderers for agent exploration.

mod api;
mod cargo;
pub(crate) mod compact;
mod context;
mod dependency;
mod format;
mod fzf_query;
mod guide;
mod hits;
mod ingest;
mod item_query;
mod limits;
mod owner;
mod owner_view;
pub(crate) mod policy;
mod prime;
mod prime_support;
mod query;
mod recency;
mod scope;
mod syntax_query;

pub use api::{
    RustSearchOptions, RustSearchViewRequest, render_rust_project_harness_search_view_with_config,
};
pub use ingest::render_rust_project_harness_search_ingest_with_config;
pub use prime::{
    render_rust_project_harness_search_prime, render_rust_project_harness_search_prime_with_config,
};
