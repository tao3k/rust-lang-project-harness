use std::future::Future;

pub struct AgentRuntime {
    handle: tokio::runtime::Handle,
}

impl AgentRuntime {
    /// Tokio runtime boundary: owns the runtime thread model for agent work.
    pub fn build_multi_thread_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("runtime")
    }

    pub fn from_handle(handle: tokio::runtime::Handle) -> Self {
        Self { handle }
    }

    /// Tokio runtime boundary: attaches task accounting and shutdown ownership.
    pub fn spawn<F>(&self, task: F) -> tokio::task::JoinHandle<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.handle.spawn(task)
    }

    /// Tokio runtime boundary: isolates blocking work from runtime workers.
    pub fn spawn_blocking<F, T>(&self, work: F) -> tokio::task::JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        tokio::task::spawn_blocking(work)
    }
}

pub struct AgentRunner {
    runtime: AgentRuntime,
}

impl AgentRunner {
    pub fn new(runtime: AgentRuntime) -> Self {
        Self { runtime }
    }

    pub fn start_background(&self) -> tokio::task::JoinHandle<()> {
        self.runtime.spawn(async move {
            publish_heartbeat().await;
        })
    }

    pub async fn index_payload(&self, payload: Vec<u8>) -> usize {
        self.runtime
            .spawn_blocking(move || payload.len())
            .await
            .unwrap_or(0)
    }
}

async fn publish_heartbeat() {}
