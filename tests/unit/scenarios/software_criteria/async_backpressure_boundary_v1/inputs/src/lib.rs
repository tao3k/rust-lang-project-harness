//! Input fixture for unbounded async queue policy.

use tokio::sync::mpsc;

/// Fans out work through an unbounded queue without a readiness or capacity boundary.
pub async fn fanout(items: Vec<String>) -> usize {
    let (tx, mut rx) = mpsc::unbounded_channel();
    for item in items {
        let _ = tx.send(item);
    }
    drop(tx);

    let mut count = 0;
    while let Some(_item) = rx.recv().await {
        count += 1;
    }
    count
}
