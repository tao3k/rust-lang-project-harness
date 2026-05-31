//! Compact search protocol renderers for agent exploration.

mod api;
mod cargo;
mod context;
mod format;
mod hits;
mod ingest;
mod limits;
mod owner;
mod owner_view;
mod prime;
mod prime_support;
mod query;
mod scope;

pub use api::{
    RustSearchOptions, RustSearchViewRequest, render_rust_project_harness_search_view_with_config,
};
pub use ingest::render_rust_project_harness_search_ingest_with_config;
pub use prime::{
    render_rust_project_harness_search_prime, render_rust_project_harness_search_prime_with_config,
};
