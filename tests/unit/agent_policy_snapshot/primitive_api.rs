use std::fs;

use tempfile::TempDir;

use super::{assert_agent_snapshot, write_manifest};

#[test]
fn agent_r012_public_primitive_identifier_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r012-primitive-id");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Loads a user.\n\
         pub fn load_user(user_id: String) {}\n",
    )
    .expect("write api");

    assert_agent_snapshot(
        root,
        "AGENT-R012",
        1,
        "agent_r012_public_primitive_identifier",
    );
}
