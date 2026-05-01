use std::fs;

use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationProfileHint, build_rust_verification_profile_index,
    build_rust_verification_profile_index_with_config, default_rust_harness_config,
    render_rust_verification_profile_index, render_rust_verification_profile_index_json,
};
use tempfile::TempDir;

use crate::verification::support::{normalize_temp_root, write_manifest};

#[test]
fn verification_profile_index_reports_parser_suggested_responsibilities() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_responsibility_fixture(root);

    let index = build_rust_verification_profile_index(root).expect("profile index");
    let rendered = normalize_temp_root(&render_rust_verification_profile_index(&index), root);

    assert!(!index.is_clear());
    insta::assert_snapshot!("verification_profile_index_candidates", rendered);
}

#[test]
fn verification_profile_index_json_preserves_structured_candidates() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_api_fixture(root);

    let index = build_rust_verification_profile_index(root).expect("profile index");
    let json = render_rust_verification_profile_index_json(&index).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");

    assert_eq!(
        value["candidates"][0]["hint_path"]
            .as_str()
            .expect("hint path")
            .replace('\\', "/"),
        "src/api.rs"
    );
    assert_eq!(value["candidates"][0]["state"], "missing_profile");
}

#[test]
fn configured_profile_hint_clears_profile_index_reminder() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_api_fixture(root);

    let config = default_rust_harness_config().with_verification_profile_hint(
        RustVerificationProfileHint::new(
            "src/api.rs",
            [
                RustOwnerResponsibility::PublicApi,
                RustOwnerResponsibility::ExternalDependency,
                RustOwnerResponsibility::AvailabilityCritical,
            ],
        ),
    );
    let index =
        build_rust_verification_profile_index_with_config(root, &config).expect("profile index");

    assert!(index.is_clear());
    assert_eq!(render_rust_verification_profile_index(&index), "");
}

#[test]
fn partial_profile_hint_renders_profile_drift() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_api_fixture(root);

    let config = default_rust_harness_config().with_verification_profile_hint(
        RustVerificationProfileHint::new("src/api.rs", [RustOwnerResponsibility::PublicApi]),
    );
    let index =
        build_rust_verification_profile_index_with_config(root, &config).expect("profile index");
    let rendered = normalize_temp_root(&render_rust_verification_profile_index(&index), root);

    assert!(!index.is_clear());
    insta::assert_snapshot!("verification_profile_index_drift", rendered);
}

#[test]
fn workspace_profile_hint_paths_are_project_relative() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/api\"]\n",
    )
    .expect("write workspace manifest");
    let package_root = root.join("crates/api");
    fs::create_dir_all(&package_root).expect("create member");
    write_api_fixture(&package_root);

    let index = build_rust_verification_profile_index(root).expect("profile index");
    let hints = index.active_profile_hints();

    assert_eq!(hints.len(), 1);
    assert_eq!(
        hints[0].owner_path,
        std::path::PathBuf::from("crates/api/src/api.rs")
    );
}

fn write_responsibility_fixture(root: &std::path::Path) {
    write_manifest(root, "verification-profile-index");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod api;\nmod auth;\nmod search;\nmod storage;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! API owner.\nuse axum::Router;\npub fn router() -> Router { unimplemented!() }\n",
    )
    .expect("write api");
    fs::write(
        root.join("src/storage.rs"),
        "//! Storage owner.\nuse std::fs;\npub fn load() -> String { String::new() }\n",
    )
    .expect("write storage");
    fs::write(
        root.join("src/auth.rs"),
        "//! Auth owner.\nuse sha2::Digest;\npub fn check_token(token: String) -> bool { !token.is_empty() }\n",
    )
    .expect("write auth");
    fs::write(
        root.join("src/search.rs"),
        "//! Search owner.\nuse rayon::prelude::*;\npub fn search() {}\n",
    )
    .expect("write search");
}

fn write_api_fixture(root: &std::path::Path) {
    write_manifest(root, "verification-profile-api");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! API owner.\nuse axum::Router;\npub fn router() -> Router { unimplemented!() }\n",
    )
    .expect("write api");
}
