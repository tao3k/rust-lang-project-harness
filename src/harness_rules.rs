//! Embedded harness rule rendering for downstream test fixtures.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Source-embedded harness rule list.
pub const RUST_HARNESS_RULES_MD: &str = include_str!("harness-rules.md");

/// Return the source-embedded harness rule list.
#[must_use]
pub const fn rust_harness_rules_markdown() -> &'static str {
    RUST_HARNESS_RULES_MD
}

/// Render the source-embedded Rust harness rules as generated markdown.
#[must_use]
pub fn render_rust_harness_rules_markdown() -> String {
    let mut output = String::from(
        "# rust-lang-project-harness\n\n\
         ## Harness Rules\n\n\
         Generated from embedded `src/harness-rules.md`.\n\n",
    );
    for line in RUST_HARNESS_RULES_MD.lines() {
        if let Some(item) = line.strip_prefix("- ")
            && let Some((rule_id, sentence)) = item.split_once(": ")
        {
            output.push_str("- **");
            output.push_str(rule_id);
            output.push_str("**: ");
            output.push_str(sentence);
            output.push('\n');
        }
    }
    output
}

/// Write the generated harness rules into a downstream unit test directory.
///
/// Downstream crates can call this from `build.rs` after adding the harness as
/// a build dependency, then commit or assert the generated unit fixture.
pub fn write_rust_harness_rules_to_unit_tests(
    unit_test_dir: impl AsRef<Path>,
) -> io::Result<PathBuf> {
    let output_path = unit_test_dir.as_ref().join("harness-rules.generated.md");
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, render_rust_harness_rules_markdown())?;
    Ok(output_path)
}
