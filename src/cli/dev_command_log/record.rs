use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use super::command::NormalizedCommand;

pub(crate) struct ActiveCommandLog {
    pub(crate) argv: Vec<String>,
    pub(crate) binary: String,
    pub(crate) command: NormalizedCommand,
    pub(crate) context_source: String,
    pub(crate) cwd: PathBuf,
    pub(crate) event_id: String,
    pub(crate) hook_run_id: Option<String>,
    pub(crate) log_file: PathBuf,
    pub(crate) parent_event_id: Option<String>,
    pub(crate) project_root: PathBuf,
    pub(crate) project_root_hash: String,
    pub(crate) session_id: String,
    pub(crate) session_ordinal: u64,
    pub(crate) started_at_instant: Instant,
    pub(crate) started_at_system: SystemTime,
}
