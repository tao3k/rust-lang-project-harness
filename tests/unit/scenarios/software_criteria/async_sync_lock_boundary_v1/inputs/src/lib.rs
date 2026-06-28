use std::sync::RwLock;

pub struct SharedState {
    value: RwLock<usize>,
}

impl SharedState {
    pub async fn refresh(&self) -> usize {
        let guard = self.value.read().unwrap();
        tokio::task::yield_now().await;
        *guard
    }
}
