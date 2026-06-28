pub struct AgentRunner;

impl AgentRunner {
    pub fn start_background(&self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            publish_heartbeat().await;
        })
    }

    pub async fn index_payload(&self, payload: Vec<u8>) -> usize {
        tokio::task::spawn_blocking(move || payload.len())
            .await
            .unwrap_or(0)
    }

    pub fn build_local_runtime(&self) -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
    }
}

async fn publish_heartbeat() {}
