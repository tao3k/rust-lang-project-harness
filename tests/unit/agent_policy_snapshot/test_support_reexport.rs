use std::fs;

use tempfile::TempDir;

use super::{assert_agent_snapshot, write_manifest};

#[test]
fn agent_r014_test_support_reexport_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r014-test-support");
    write_unused_test_support_reexport_fixture(root);

    assert_agent_snapshot(root, "AGENT-R014", 1, "agent_r014_test_support_reexport");
}

fn write_unused_test_support_reexport_fixture(root: &std::path::Path) {
    fs::create_dir(root.join("src")).expect("create src dir");
    fs::create_dir_all(root.join("tests/unit/search/service")).expect("create tests");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\npub mod domain;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain test types.\n\
         pub struct LocalType;\n\
         pub struct SupportType;\n\
         pub struct UnusedType;\n",
    )
    .expect("write domain");
    fs::write(
        root.join("tests/unit/search/service/support.rs"),
        "pub(super) use crate::domain::{LocalType, SupportType, UnusedType};\n\
         pub(super) fn helper(value: LocalType) -> LocalType { value }\n",
    )
    .expect("write support");
    fs::write(
        root.join("tests/unit/search/service/consumer.rs"),
        "use super::support::{SupportType, helper};\n\
         fn smoke(value: SupportType) { let _ = helper(crate::domain::LocalType); let _ = value; }\n",
    )
    .expect("write consumer");
}
