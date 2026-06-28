//! Expected fixture facade for a supervised Tokio worker.

mod worker;

pub use worker::{MetricsWorker, start_metrics_worker};
