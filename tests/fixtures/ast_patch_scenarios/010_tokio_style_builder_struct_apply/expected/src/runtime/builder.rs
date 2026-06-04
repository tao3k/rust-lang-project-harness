#[cfg(feature = "rt")]
#[derive(Debug, Clone)]
pub(crate) struct Builder {
    pub(crate) worker_threads: usize,
    pub(crate) enable_io: bool,
    pub(crate) thread_name: Option<String>,
}

impl Builder {
    pub(crate) fn new() -> Self {
        Self {
            worker_threads: 1,
            enable_io: false,
        }
    }
}
