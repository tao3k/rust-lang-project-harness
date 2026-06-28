use std::time::Duration;

pub async fn summarize(paths: Vec<String>) -> Result<usize, tokio::task::JoinError> {
    tokio::task::spawn_blocking(move || {
        paths
            .into_iter()
            .map(|path| {
                std::thread::sleep(Duration::from_millis(2));
                path.len()
            })
            .sum()
    })
    .await
}
