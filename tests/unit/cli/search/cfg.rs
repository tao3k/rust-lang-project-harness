#![allow(unused_imports)]

use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::cli::support::{
    normalize_temp_root, run_cli, run_search, run_search_with_stdin, write_manifest,
    write_search_fixture,
};

#[test]
fn cli_search_cfg_reads_manifest_facts() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"cli-search-cfg\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [features]\n\
         json = []\n\n\
         [lints.rust]\n\
         unexpected_cfgs = { level = \"warn\", check-cfg = ['cfg(loom)'] }\n\n\
         [target.'cfg(loom)'.dev-dependencies]\n\
         loom = { version = \"0.7\", features = [\"futures\"] }\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let loom = run_search(root, &["cfg", "loom"]);
    assert!(
        loom.starts_with("[search-cfg] q=loom pkg=. cfg=2 dep=1 own=0"),
        "{loom}"
    );
    assert!(
        loom.contains("|cfg loom declared_in=lints.rust.unexpected_cfgs expr=cfg(loom) source=manifest manager=cargo"),
        "{loom}"
    );
    assert!(
        loom.contains(
            "|cfg loom declared_in=target.dependencies expr=cfg(loom) source=manifest manager=cargo"
        ),
        "{loom}"
    );
    assert!(
        loom.contains(
            "|dep loom import=loom pkg=loom version=^0.7 kind=dev opt=false source=manifest manager=cargo target=cfg(loom) feat=futures"
        ),
        "{loom}"
    );
    assert!(
        loom.contains("|next text:cfg(loom)(scope=src),text:loom(scope=tests)"),
        "{loom}"
    );

    let feature = run_search(root, &["cfg", "json"]);
    assert!(
        feature.starts_with("[search-cfg] q=json pkg=. cfg=1 dep=0 own=0"),
        "{feature}"
    );
    assert!(
        feature.contains("|cfg feature:json declared_in=features expr=cfg(feature=\"json\") source=manifest manager=cargo"),
        "{feature}"
    );
}
