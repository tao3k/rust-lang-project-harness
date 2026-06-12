use std::fs;

use tempfile::TempDir;

use crate::cli::support::{run_search, write_manifest};

#[test]
fn cli_search_env_reports_toolchain_and_cfg_witnesses() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-search-env");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let toolchain = run_search(root, &["env", "toolchain"]);
    assert!(
        toolchain.starts_with("[search-env] q=toolchain pkg=."),
        "{toolchain}"
    );
    assert!(
        toolchain.contains("|env rustcVersion=")
            && toolchain.contains("source=rustc-version evidenceGrade=witness"),
        "{toolchain}"
    );
    assert!(
        toolchain.contains("|env cargoManifest edition=2024 resolver=- features=0 source=manifest manager=cargo evidenceGrade=fact"),
        "{toolchain}"
    );
    assert!(
        toolchain.contains(
            "|quality status=partial missing=cargo-metadata,resolved-features next=env:cfg"
        ),
        "{toolchain}"
    );

    let cfg = run_search(root, &["env", "cfg"]);
    assert!(cfg.starts_with("[search-env] q=cfg pkg=."), "{cfg}");
    assert!(
        cfg.contains("source=rustc-print-cfg evidenceGrade=witness"),
        "{cfg}"
    );
    assert!(
        cfg.contains(
            "|quality status=partial missing=cargo-metadata,resolved-feature-cfg next=cfg:<name>"
        ),
        "{cfg}"
    );
}

#[test]
fn cli_search_code_comments_labels_claims_without_semantic_verdict() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-search-code-comments");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Module promise.\n// Runtime claim.\npub fn value() -> usize { 1 }\n",
    )
    .expect("write lib");

    let output = run_search(root, &["code", "comments", "--owner", "src/lib.rs"]);

    assert!(
        output.starts_with("[search-code] q=comments pkg=. claim=2 fact=0 witness=0"),
        "{output}"
    );
    assert!(
        output.contains("|claim kind=module-doc-comment owner=src/lib.rs line=1 evidenceGrade=claim evidence=comment verdict=unverified text=Module_promise."),
        "{output}"
    );
    assert!(
        output.contains("|claim kind=line-comment owner=src/lib.rs line=2 evidenceGrade=claim evidence=comment verdict=unverified text=Runtime_claim."),
        "{output}"
    );
    assert!(
        output.contains(
            "|quality status=partial missing=parser-verdict,witness next=owner:src/lib.rs"
        ),
        "{output}"
    );
}

#[test]
fn cli_search_extension_tokio_uses_manifest_and_source_derived_boundary_facts() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"cli-search-extension-tokio\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [dependencies]\n\
         tokio = { version = \"1\", features = [\"rt\", \"time\", \"process\"] }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "use std::time::Duration;\nuse tokio::time::timeout;\n\npub async fn bounded() {\n    let _ = timeout(Duration::from_secs(1), async {}).await;\n}\n",
    )
    .expect("write lib");

    let output = run_search(root, &["extension", "tokio"]);

    assert!(
        output.starts_with("[search-extension] q=tokio pkg=. extension=tokio dep=1 own=1"),
        "{output}"
    );
    assert!(
        output.contains("|extension tokio status=activated source=manifest evidenceGrade=fact"),
        "{output}"
    );
    assert!(
        output.contains("|owner src/lib.rs hit_kind=extension-usage extension=tokio"),
        "{output}"
    );
    assert!(
        output.contains("|extension-guidance dep=tokio usageLevel=capability_boundary engineeringBoundary=present ownerUsage=1"),
        "{output}"
    );
    assert!(
        output.contains("source=provider-capability-catalog evidenceGrade=fact"),
        "{output}"
    );
}
