//! Worker implementation that silently detaches spawned work.

use std::time::Duration;
use tokio::sync::mpsc;

/// Start a metrics worker without exposing its task lifecycle.
pub fn start_metrics_worker(mut receiver: mpsc::Receiver<u64>) {
    tokio::spawn(async move {
        while let Some(value) = receiver.recv().await {
            tokio::time::sleep(Duration::from_millis(value)).await;
        }
    });
}
