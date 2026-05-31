use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::model::{CODEX_POLICY_PATH, Profile};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub(super) struct CodexHookPolicy {
    pub(super) profiles: ProfilePolicies,
    pub(super) global: GlobalPolicy,
}

impl CodexHookPolicy {
    pub(super) fn load(root: &Path) -> Self {
        fs::read_to_string(root.join(CODEX_POLICY_PATH))
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    pub(super) fn profile(&self, profile: Profile) -> &ProfilePolicy {
        match profile {
            Profile::Rust => &self.profiles.rust,
            Profile::TypeScript => &self.profiles.typescript,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub(super) struct ProfilePolicies {
    pub(super) rust: ProfilePolicy,
    pub(super) typescript: ProfilePolicy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub(super) struct ProfilePolicy {
    pub(super) enabled: bool,
    pub(super) prime_required_before_edit: bool,
    pub(super) raw_search_requires_ingest: bool,
    pub(super) changed_check_required: bool,
}

impl ProfilePolicy {
    fn enabled() -> Self {
        Self {
            enabled: true,
            prime_required_before_edit: true,
            raw_search_requires_ingest: true,
            changed_check_required: true,
        }
    }
}

impl Default for ProfilePolicy {
    fn default() -> Self {
        Self::enabled()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub(super) struct GlobalPolicy {
    pub(super) raw_ast_grep_blocked: bool,
    pub(super) exact_file_edit_exception: bool,
    pub(super) docs_only_exception: bool,
}

impl Default for GlobalPolicy {
    fn default() -> Self {
        Self {
            raw_ast_grep_blocked: true,
            exact_file_edit_exception: true,
            docs_only_exception: true,
        }
    }
}
