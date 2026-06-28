//! Worker implementation that exposes spawned task ownership.

use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Handle that owns the metrics worker task lifecycle.
pub struct MetricsWorker {
    handle: JoinHandle<()>,
}

impl MetricsWorker {
    /// Wait for the metrics worker task to finish.
    pub async fn join(self) -> Result<(), tokio::task::JoinError> {
        self.handle.await
    }

    /// Abort the metrics worker task during shutdown.
    pub fn abort(&self) {
        self.handle.abort();
    }
}

/// Start a metrics worker and return its lifecycle handle.
pub fn start_metrics_worker(mut receiver: mpsc::Receiver<u64>) -> MetricsWorker {
    let handle = tokio::spawn(async move {
        while let Some(value) = receiver.recv().await {
            tokio::time::sleep(Duration::from_millis(value)).await;
        }
    });
    MetricsWorker { handle }
}
