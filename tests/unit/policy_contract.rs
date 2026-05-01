use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use rust_lang_project_harness::{
    RustDiagnosticSeverity, default_rust_harness_config, render_rust_project_harness,
    run_rust_project_harness, rust_agent_policy_rules,
};

#[test]
fn default_policy_blocks_only_warning_and_error() {
    let config = default_rust_harness_config();

    assert_eq!(
        config.blocking_severities,
        BTreeSet::from([
            RustDiagnosticSeverity::Warning,
            RustDiagnosticSeverity::Error,
        ])
    );
}

#[test]
fn agent_policy_rules_are_non_blocking_advice() {
    for rule in rust_agent_policy_rules() {
        assert_eq!(
            rule.severity,
            RustDiagnosticSeverity::Info,
            "{}",
            rule.rule_id
        );
    }
}

#[test]
fn crate_is_clean_under_its_own_project_harness() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let report = run_rust_project_harness(&root).expect("run self harness");
    let rendered = render_rust_project_harness(&report);

    assert!(report.is_clean(), "{rendered}");
    assert!(rendered.contains("No blocking issues found."));
}

#[test]
fn policy_rules_consume_parser_facts_not_syn_directly() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut offenders = Vec::new();
    for path in rust_files_under(&root.join("src/rules")) {
        let source = fs::read_to_string(&path).expect("read rule source");
        if source.lines().any(|line| {
            let line = line.trim_start();
            line.starts_with("use syn") || line.contains("syn::")
        }) {
            offenders.push(relative_path(&root, &path));
        }
    }

    assert!(
        offenders.is_empty(),
        "policy rule modules must consume src/parser facts, not syn directly: {offenders:?}"
    );
}

#[test]
fn syn_parse_file_entrypoint_stays_inside_parser_module() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parser_entrypoints = rust_files_under(&root.join("src"))
        .into_iter()
        .filter(|path| {
            fs::read_to_string(path)
                .expect("read Rust source")
                .contains("syn::parse_file")
        })
        .map(|path| relative_path(&root, &path))
        .collect::<Vec<_>>();

    assert_eq!(parser_entrypoints, vec!["src/parser/parsed_module.rs"]);
}

#[test]
fn parsed_module_does_not_expose_syn_file_to_policy_rules() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parsed_module =
        fs::read_to_string(root.join("src/parser/parsed_module.rs")).expect("read parsed module");

    assert!(!parsed_module.contains("pub syntax: Option<syn::File>"));
}

#[test]
fn cargo_manifest_parser_lives_under_parser_module() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parser_source = fs::read_to_string(root.join("src/parser/cargo_manifest.rs"))
        .expect("read cargo manifest parser");
    assert!(parser_source.contains("toml::from_str::<CargoManifestToml>"));

    let mut offenders = Vec::new();
    for path in rust_files_under(&root.join("src")) {
        let relative = relative_path(&root, &path);
        if relative == "src/parser/cargo_manifest.rs" {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read Rust source");
        if source.contains("CargoManifestToml")
            || source.contains("toml::from_str::<CargoManifestToml>")
        {
            offenders.push(relative);
        }
    }

    assert!(
        offenders.is_empty(),
        "Cargo manifest TOML parsing must live in src/parser/cargo_manifest.rs: {offenders:?}"
    );
}

#[test]
fn library_target_mounts_source_backed_self_apply_gate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_rs = fs::read_to_string(root.join("src/lib.rs")).expect("read src/lib.rs");
    let self_policy =
        fs::read_to_string(root.join("src/self_policy.rs")).expect("read src/self_policy.rs");

    assert!(!lib_rs.contains("rust_project_harness_source_gate!"));
    assert!(self_policy.contains("rust_project_harness_cargo_test_gate!()"));
}

#[test]
fn crate_facade_keeps_macro_implementation_out_of_lib_rs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_rs = fs::read_to_string(root.join("src/lib.rs")).expect("read src/lib.rs");
    let macros_rs = fs::read_to_string(root.join("src/macros.rs")).expect("read src/macros.rs");

    assert!(!lib_rs.contains("macro_rules!"));
    assert!(macros_rs.contains("macro_rules! rust_project_harness_gate"));
    assert!(macros_rs.contains("macro_rules! rust_project_harness_cargo_test_gate"));
}

#[test]
fn root_test_target_mounts_direct_project_gate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let unit_test =
        fs::read_to_string(root.join("tests/unit_test.rs")).expect("read tests/unit_test.rs");

    assert!(unit_test.contains("rust_project_harness_gate!()"));
}

fn rust_files_under(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust_files(root, &mut files);
    files.sort();
    files
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read directory") {
        let path = entry.expect("read directory entry").path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("path under repository root")
        .to_string_lossy()
        .replace('\\', "/")
}
