use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::model::{
    CODEX_STATE_DIR, Profile, rust_config_files, rust_roots, ts_config_files, ts_extensions,
    ts_roots,
};
use super::policy::CodexHookPolicy;

#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct ProjectProfiles {
    pub(super) rust: DetectedProfile,
    pub(super) typescript: DetectedProfile,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct DetectedProfile {
    pub(super) enabled: bool,
    command: String,
    source_extensions: Vec<String>,
    source_roots: Vec<String>,
    config_files: Vec<String>,
}

impl ProjectProfiles {
    pub(super) fn detect(root: &Path, policy: &CodexHookPolicy) -> Self {
        Self {
            rust: DetectedProfile {
                enabled: policy.profiles.rust.enabled && rust_profile_present(root),
                command: "rs-harness".to_string(),
                source_extensions: vec![".rs".to_string()],
                source_roots: strings(rust_roots()),
                config_files: strings(rust_config_files()),
            },
            typescript: DetectedProfile {
                enabled: policy.profiles.typescript.enabled && typescript_profile_present(root),
                command: "ts-harness".to_string(),
                source_extensions: strings(ts_extensions()),
                source_roots: strings(ts_roots()),
                config_files: strings(ts_config_files()),
            },
        }
    }

    pub(super) fn save(&self, root: &Path) -> Result<(), String> {
        let dir = root.join(CODEX_STATE_DIR);
        fs::create_dir_all(&dir)
            .map_err(|error| format!("failed to create codex hook state dir: {error}"))?;
        let content = serde_json::to_string_pretty(self)
            .map_err(|error| format!("failed to render codex project profiles: {error}"))?;
        fs::write(dir.join("project.json"), content)
            .map_err(|error| format!("failed to write codex project profiles: {error}"))
    }

    pub(super) fn enabled_profiles(&self) -> BTreeSet<Profile> {
        [
            (self.rust.enabled, Profile::Rust),
            (self.typescript.enabled, Profile::TypeScript),
        ]
        .into_iter()
        .filter_map(|(enabled, profile)| enabled.then_some(profile))
        .collect()
    }

    pub(super) fn session_context(&self) -> &'static str {
        match (self.rust.enabled, self.typescript.enabled) {
            (true, true) => {
                "Codex harness profiles detected: rust, typescript. Use profile-specific prime and ingest only for matching source/config files."
            }
            (true, false) => {
                "Codex harness profile detected: rust. Use `rs-harness search prime` for complex Rust tasks."
            }
            (false, true) => {
                "Codex harness profile detected: typescript. Use `ts-harness search prime` for complex TS/JS tasks."
            }
            (false, false) => {
                "No Rust or TS/JS harness profile detected; do not enforce harness flow."
            }
        }
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn rust_profile_present(root: &Path) -> bool {
    root.join("Cargo.toml").exists() || root.join("src").exists()
}

fn typescript_profile_present(root: &Path) -> bool {
    root.join("package.json").exists()
        && (root.join("tsconfig.json").exists()
            || root.join("jsconfig.json").exists()
            || root.join("src").exists()
            || root.join("apps").exists()
            || root.join("packages").exists())
}
