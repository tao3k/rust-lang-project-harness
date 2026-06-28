//! Expected fixture for unbounded async queue policy.

use tokio::sync::mpsc;

/// Fans out work through a bounded queue with an explicit capacity boundary.
pub async fn fanout(items: Vec<String>) -> usize {
    let (tx, mut rx) = mpsc::channel(64);
    for item in items {
        let _ = tx.send(item).await;
    }
    drop(tx);

    let mut count = 0;
    while let Some(_item) = rx.recv().await {
        count += 1;
    }
    count
}
