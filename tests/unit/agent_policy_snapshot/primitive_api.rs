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

#[test]
fn agent_r018_public_flag_params_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r018-flag-params");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Loads users.\n\
         pub fn load_users(include_inactive: bool, allow_cache: Option<bool>, limit: usize) {}\n",
    )
    .expect("write api");

    assert_agent_snapshot(root, "AGENT-R018", 1, "agent_r018_public_flag_params");
}

#[test]
fn agent_r019_public_positional_surface_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r019-positional-surface");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Client handle.\n\
         pub struct Client;\n\
         impl Client {\n\
         \t/// Creates a client.\n\
         \tpub fn new(endpoint: String, token: String, retries: usize, timeout_ms: u64, batch_size: usize) -> Self { Self }\n\
         }\n",
    )
    .expect("write api");

    assert_agent_snapshot(
        root,
        "AGENT-R019",
        1,
        "agent_r019_public_positional_surface",
    );
}

#[test]
fn agent_r020_public_data_shape_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r020-data-shape");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// User profile crossing a public boundary.\n\
         pub struct UserProfile {\n\
         \tpub user_id: String,\n\
         \tpub session_token: String,\n\
         \tpub timeout_ms: u64,\n\
         \tpub include_inactive: bool,\n\
         }\n",
    )
    .expect("write api");

    assert_agent_snapshot(root, "AGENT-R020", 1, "agent_r020_public_data_shape");
}

#[test]
fn agent_r021_public_enum_payload_shape_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r021-enum-payload");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Domain event emitted by the API.\n\
         pub enum DomainEvent {\n\
         \t/// User data was loaded.\n\
         \tUserLoaded { user_id: String, request_id: String, include_inactive: bool },\n\
         }\n",
    )
    .expect("write api");

    assert_agent_snapshot(
        root,
        "AGENT-R021",
        1,
        "agent_r021_public_enum_payload_shape",
    );
}

#[test]
fn agent_r022_public_generic_data_bounds_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r022-generic-data-bounds");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Generic cache entry.\n\
         pub struct CacheEntry<T: Clone + std::fmt::Debug>\n\
         where\n\
         \tT: Default,\n\
         {\n\
         \tpub value: T,\n\
         }\n",
    )
    .expect("write api");

    assert_agent_snapshot(
        root,
        "AGENT-R022",
        1,
        "agent_r022_public_generic_data_bounds",
    );
}

#[test]
fn agent_r023_public_tuple_api_surface_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r023-tuple-api-surface");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Loads a page of users.\n\
         pub fn load_users(cursor: (String, usize, bool)) -> Result<(String, usize), LoadError> { todo!() }\n",
    )
    .expect("write api");

    assert_agent_snapshot(root, "AGENT-R023", 2, "agent_r023_public_tuple_api_surface");
}

#[test]
fn agent_r024_public_enum_tuple_payload_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r024-enum-tuple-payload");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Domain event emitted by the API.\n\
         pub enum DomainEvent {\n\
         \t/// User data was loaded.\n\
         \tUserLoaded(String, usize, bool),\n\
         }\n",
    )
    .expect("write api");

    assert_agent_snapshot(
        root,
        "AGENT-R024",
        1,
        "agent_r024_public_enum_tuple_payload",
    );
}
