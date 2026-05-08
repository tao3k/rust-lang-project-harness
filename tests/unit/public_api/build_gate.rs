use std::fs;
use std::path::Path;

use rust_lang_project_harness::assert_rust_project_harness_build_clean;
use tempfile::TempDir;

#[test]
fn build_gate_assertion_does_not_promote_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_advice_only_project(root, "advice-build-gate");

    assert_rust_project_harness_build_clean(root);
}

#[test]
fn build_gate_assertion_blocks_configured_findings_before_libtest_filter() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_oversized_project(root, "oversized-build-gate");

    let panic = std::panic::catch_unwind(|| {
        assert_rust_project_harness_build_clean(root);
    })
    .expect_err("source bloat should fail build gate assertion");
    let normalized = normalize_temp_root(&panic_message(panic), root);

    assert!(normalized.contains("RUST-MOD-R002"), "{normalized}");
    assert!(
        normalized.contains("Source file carries too many responsibilities"),
        "{normalized}"
    );
}

fn write_advice_only_project(root: &Path, name: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "mod owned;\npub use owned::public_api;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/owned.rs"), "pub fn public_api() {}\n").expect("write owned module");
}

fn write_oversized_project(root: &Path, name: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "mod large;\n").expect("write lib");
    let large_source = (0..700)
        .map(|index| format!("fn internal_{index}() -> usize {{ {index} }}\n"))
        .collect::<String>();
    fs::write(root.join("src/large.rs"), large_source).expect("write large module");
}

fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_owned();
    }
    "<non-string panic>".to_owned()
}
