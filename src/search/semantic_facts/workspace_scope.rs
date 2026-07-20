//! Provider-owned Cargo workspace membership for pre-rank candidate admission.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::parser::parse_cargo_manifest;

use super::contract::{LANGUAGE_ID, PROVIDER_ID};
use super::dependency_topology::cargo_package_roots;

const SCHEMA_ID: &str = "agent.semantic-protocols.semantic-workspace-scope";
const SCHEMA_VERSION: &str = "1";
const PACKAGE_MANAGER: &str = "cargo";
const MANIFEST_FILE: &str = "Cargo.toml";
const LOCKFILE_FILE: &str = "Cargo.lock";
const SOURCE_EXTENSIONS: &[&str] = &[".rs"];

/// Renders canonical Cargo workspace membership for candidate admission.
pub fn render_rust_project_harness_workspace_scope_json(
    project_root: &Path,
) -> Result<String, String> {
    let packet = build_workspace_scope_packet(project_root)?;
    serde_json::to_string_pretty(&packet)
        .map(|mut text| {
            text.push('\n');
            text
        })
        .map_err(|error| format!("failed to render workspace scope JSON: {error}"))
}

fn build_workspace_scope_packet(project_root: &Path) -> Result<Value, String> {
    let discovery_root = canonical_path(project_root, "workspace discovery root")?;
    let root_manifest = parse_cargo_manifest(&discovery_root);
    let package_roots = cargo_package_roots(&discovery_root, root_manifest.package_name.is_some());
    let mut packages = package_roots
        .into_iter()
        .map(|root| canonical_package(&root))
        .collect::<Result<Vec<_>, _>>()?;
    packages.sort_by(|left, right| {
        left.get("root")
            .and_then(Value::as_str)
            .cmp(&right.get("root").and_then(Value::as_str))
    });
    if packages.is_empty() {
        return Err(format!(
            "Cargo workspace at {} has no package members",
            discovery_root.display()
        ));
    }

    let admitted_roots = packages
        .iter()
        .filter_map(|package| package.get("root").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let anchors = workspace_anchors(&discovery_root, &packages)?;
    let workspace_id = workspace_id(&discovery_root);
    let fingerprint = packet_fingerprint(
        &workspace_id,
        &discovery_root,
        &anchors,
        &packages,
        &admitted_roots,
    )?;

    Ok(json!({
        "schemaId": SCHEMA_ID,
        "schemaVersion": SCHEMA_VERSION,
        "workspaceId": workspace_id,
        "languageId": LANGUAGE_ID,
        "providerId": PROVIDER_ID,
        "packageManager": PACKAGE_MANAGER,
        "sourceExtensions": SOURCE_EXTENSIONS,
        "discoveryRoot": display_absolute(&discovery_root),
        "anchors": anchors,
        "packages": packages,
        "admittedRoots": admitted_roots,
        "fingerprint": fingerprint,
    }))
}

fn canonical_package(root: &Path) -> Result<Value, String> {
    let root = canonical_path(root, "Cargo package root")?;
    let manifest_path = canonical_path(&root.join(MANIFEST_FILE), "Cargo package manifest")?;
    let manifest = parse_cargo_manifest(&root);
    let name = manifest.package_name.ok_or_else(|| {
        format!(
            "Cargo workspace member {} has no [package].name",
            manifest_path.display()
        )
    })?;
    let root_text = display_absolute(&root);
    Ok(json!({
        "packageId": format!("cargo:{name}:{root_text}"),
        "name": name,
        "root": root_text,
        "manifestPath": display_absolute(&manifest_path),
        "languageId": LANGUAGE_ID,
    }))
}

fn workspace_anchors(discovery_root: &Path, packages: &[Value]) -> Result<Vec<Value>, String> {
    let mut paths = BTreeSet::<(String, PathBuf)>::new();
    let root_manifest = discovery_root.join(MANIFEST_FILE);
    if root_manifest.is_file() {
        paths.insert((
            "cargo-manifest".to_string(),
            canonical_path(&root_manifest, "Cargo manifest")?,
        ));
    }
    for package in packages {
        let Some(manifest_path) = package.get("manifestPath").and_then(Value::as_str) else {
            continue;
        };
        paths.insert(("cargo-manifest".to_string(), PathBuf::from(manifest_path)));
    }
    let lockfile = discovery_root.join(LOCKFILE_FILE);
    if lockfile.is_file() {
        paths.insert((
            "cargo-lock".to_string(),
            canonical_path(&lockfile, "Cargo lockfile")?,
        ));
    }
    paths
        .into_iter()
        .map(|(kind, path)| {
            Ok(json!({
                "kind": kind,
                "path": display_absolute(&path),
                "sha256": file_sha256(&path)?,
            }))
        })
        .collect()
}

fn packet_fingerprint(
    workspace_id: &str,
    discovery_root: &Path,
    anchors: &[Value],
    packages: &[Value],
    admitted_roots: &[String],
) -> Result<String, String> {
    let payload = json!({
        "workspaceId": workspace_id,
        "languageId": LANGUAGE_ID,
        "providerId": PROVIDER_ID,
        "packageManager": PACKAGE_MANAGER,
        "sourceExtensions": SOURCE_EXTENSIONS,
        "discoveryRoot": display_absolute(discovery_root),
        "anchors": anchors,
        "packages": packages,
        "admittedRoots": admitted_roots,
    });
    let bytes = serde_json::to_vec(&payload)
        .map_err(|error| format!("failed to fingerprint workspace scope: {error}"))?;
    Ok(sha256(&bytes))
}

fn workspace_id(discovery_root: &Path) -> String {
    format!("cargo:{}", display_absolute(discovery_root))
}

fn canonical_path(path: &Path, label: &str) -> Result<PathBuf, String> {
    fs::canonicalize(path)
        .map_err(|error| format!("failed to resolve {label} {}: {error}", path.display()))
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "failed to read workspace anchor {}: {error}",
            path.display()
        )
    })?;
    Ok(sha256(&bytes))
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut rendered = String::from("sha256:");
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut rendered, "{byte:02x}");
    }
    rendered
}

fn display_absolute(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
