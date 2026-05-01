use std::fs;
use std::path::Path;

use rust_lang_project_harness::render_rust_project_harness_agent_snapshot;
use tempfile::TempDir;

#[test]
fn agent_reasoning_tree_snapshot_groups_owner_branches() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-reasoning-snapshot");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::create_dir_all(root.join("src/alt")).expect("create alternate source tree");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod domain;\n#[path = \"alt/custom.rs\"]\nmod custom;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain branch.\nmod leaf;\ninclude!(\"domain/shard.rs\");\n",
    )
    .expect("write domain");
    fs::write(root.join("src/domain/leaf.rs"), "//! Domain leaf.\n").expect("write leaf");
    fs::write(root.join("src/alt/custom.rs"), "//! Custom path child.\n").expect("write custom");
    fs::write(root.join("src/domain/shard.rs"), "//! Included shard.\n").expect("write shard");

    let rendered = render_rust_project_harness_agent_snapshot(root).expect("render snapshot");

    insta::assert_snapshot!(
        "agent_reasoning_tree_snapshot",
        normalize_temp_root(&rendered, root)
    );
}

fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}

fn write_manifest(root: &Path, name: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("write manifest");
}
