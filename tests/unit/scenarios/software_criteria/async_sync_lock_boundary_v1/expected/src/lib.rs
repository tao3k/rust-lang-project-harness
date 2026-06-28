use std::sync::RwLock;

pub struct SharedState {
    value: RwLock<usize>,
}

impl SharedState {
    pub async fn refresh(&self) -> usize {
        let value = *self.value.read().unwrap();
        tokio::task::yield_now().await;
        value
    }
}
