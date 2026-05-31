use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::model::{CODEX_STATE_DIR, Profile};

#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct HookState {
    #[serde(default)]
    turn_id: Option<String>,
    #[serde(default)]
    pub(super) rust: ProfileState,
    #[serde(default)]
    pub(super) typescript: ProfileState,
    #[serde(default)]
    pub(super) subagent_results: usize,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct ProfileState {
    #[serde(default)]
    pub(super) prime_seen: bool,
    #[serde(default)]
    pub(super) changed_check_seen: bool,
    #[serde(default)]
    pub(super) dirty_files: Vec<String>,
}

impl HookState {
    pub(super) fn load(root: &Path) -> Result<Self, String> {
        let path = root.join(CODEX_STATE_DIR).join("session.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read codex hook state: {error}"))?;
        serde_json::from_str(&content)
            .map_err(|error| format!("failed to parse codex hook state: {error}"))
    }

    pub(super) fn save(&self, root: &Path) -> Result<(), String> {
        let dir = root.join(CODEX_STATE_DIR);
        fs::create_dir_all(&dir)
            .map_err(|error| format!("failed to create codex hook state dir: {error}"))?;
        let content = serde_json::to_string_pretty(self)
            .map_err(|error| format!("failed to render codex hook state: {error}"))?;
        fs::write(dir.join("session.json"), content)
            .map_err(|error| format!("failed to write codex hook state: {error}"))
    }

    pub(super) fn start_turn(&mut self, turn_id: Option<&str>) {
        let Some(turn_id) = turn_id else {
            return;
        };
        if self.turn_id.as_deref() == Some(turn_id) {
            return;
        }
        self.turn_id = Some(turn_id.to_string());
        self.rust.changed_check_seen = false;
        self.rust.dirty_files.clear();
        self.typescript.changed_check_seen = false;
        self.typescript.dirty_files.clear();
    }

    pub(super) fn profile(&self, profile: Profile) -> &ProfileState {
        match profile {
            Profile::Rust => &self.rust,
            Profile::TypeScript => &self.typescript,
        }
    }

    pub(super) fn mark_prime(&mut self, profile: Profile) {
        self.profile_mut(profile).prime_seen = true;
    }

    pub(super) fn mark_changed_check(&mut self, profile: Profile) {
        self.profile_mut(profile).changed_check_seen = true;
    }

    pub(super) fn record_dirty(&mut self, profile: Profile, files: &[String]) {
        if files.is_empty() {
            return;
        }
        let state = self.profile_mut(profile);
        let mut merged = state.dirty_files.iter().cloned().collect::<BTreeSet<_>>();
        merged.extend(files.iter().cloned());
        state.dirty_files = merged.into_iter().collect();
        state.changed_check_seen = false;
    }

    fn profile_mut(&mut self, profile: Profile) -> &mut ProfileState {
        match profile {
            Profile::Rust => &mut self.rust,
            Profile::TypeScript => &mut self.typescript,
        }
    }
}
