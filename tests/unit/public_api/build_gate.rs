use std::fs;
use std::path::Path;

use rust_lang_project_harness::{
    assert_rust_project_harness_cargo_check_clean,
    assert_rust_project_harness_cargo_check_clean_with_config, default_rust_harness_config,
};
use tempfile::TempDir;

#[test]
fn build_gate_assertion_promotes_agent_advice_for_cargo_check_feedback() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_advice_only_project(root, "advice-build-gate");

    let panic = std::panic::catch_unwind(|| {
        assert_rust_project_harness_cargo_check_clean(root);
    })
    .expect_err("agent advice should fail the cargo-check build gate");
    let normalized = normalize_temp_root(&panic_message(panic), root);

    assert!(normalized.contains("AGENT-R001"), "{normalized}");
    assert!(
        normalized.contains("add a module intent doc"),
        "{normalized}"
    );
}

#[test]
fn build_gate_assertion_allows_agent_advice_with_explicit_explanation() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_advice_only_project(root, "advice-build-gate-allowed");
    let config = default_rust_harness_config().with_cargo_check_advice_allow_explanation(
        "scope=public API cargo-check smoke; owner=build_gate test; \
         finding_category=advisory documentation findings; \
         why_safe_now=the test intentionally exercises the advice allowance branch; \
         cleanup_trigger=remove when the fixture no longer carries advisory findings",
    );

    assert_rust_project_harness_cargo_check_clean_with_config(root, &config);
}

#[test]
fn build_gate_assertion_blocks_configured_findings_before_libtest_filter() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_oversized_project(root, "oversized-build-gate");

    let panic = std::panic::catch_unwind(|| {
        assert_rust_project_harness_cargo_check_clean(root);
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
