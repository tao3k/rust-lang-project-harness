//! Query-free, parser-owned language projection for one Rust owner.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use serde_json::{Value, json};

use crate::parser::parse_rust_file;

pub(super) fn run_language_projection(
    args: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<ExitCode, String> {
    let options = LanguageProjectionOptions::parse(args)?;
    if options.help {
        println!("{}", language_projection_usage());
        return Ok(ExitCode::SUCCESS);
    }
    if !options.json {
        return Err("projection requires --json".to_string());
    }
    println!("{}", render_language_projection(&options)?);
    Ok(ExitCode::SUCCESS)
}

struct LanguageProjectionOptions {
    owner: PathBuf,
    workspace: PathBuf,
    json: bool,
    help: bool,
}

impl LanguageProjectionOptions {
    fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Self, String> {
        let args = args
            .into_iter()
            .map(|arg| {
                arg.into_string()
                    .map_err(|_| "projection arguments must be UTF-8")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut owner = None;
        let mut workspace = None;
        let mut json = false;
        let mut help = false;
        let mut index = 0;
        while let Some(argument) = args.get(index) {
            match argument.as_str() {
                "--workspace" => {
                    let value = args.get(index + 1).ok_or_else(language_projection_usage)?;
                    workspace = Some(PathBuf::from(value));
                    index += 2;
                }
                "--json" => {
                    json = true;
                    index += 1;
                }
                "--help" | "-h" => {
                    help = true;
                    index += 1;
                }
                value if value.starts_with('-') => {
                    return Err(format!("unknown projection option: {value}"));
                }
                value => {
                    if owner.replace(PathBuf::from(value)).is_some() {
                        return Err("projection accepts exactly one owner".to_string());
                    }
                    index += 1;
                }
            }
        }
        let workspace =
            workspace.unwrap_or(std::env::current_dir().map_err(|error| error.to_string())?);
        if help {
            return Ok(Self {
                owner: owner.unwrap_or_default(),
                workspace,
                json,
                help,
            });
        }
        let owner = owner.ok_or_else(language_projection_usage)?;
        validate_relative_owner(&owner)?;
        Ok(Self {
            owner,
            workspace,
            json,
            help,
        })
    }
}

fn render_language_projection(options: &LanguageProjectionOptions) -> Result<Value, String> {
    let workspace = options
        .workspace
        .canonicalize()
        .map_err(|error| format!("failed to resolve projection workspace: {error}"))?;
    let source_path = workspace.join(&options.owner);
    let source_path = source_path
        .canonicalize()
        .map_err(|error| format!("failed to resolve projection owner: {error}"))?;
    if !source_path.starts_with(&workspace)
        || source_path.extension().and_then(|value| value.to_str()) != Some("rs")
    {
        return Err("projection owner must be a Rust source inside the workspace".to_string());
    }
    let relative_path = source_path
        .strip_prefix(&workspace)
        .map_err(|error| error.to_string())?;
    let relative_path = project_path(relative_path);
    let module = parse_rust_file(&source_path);
    let source_id = format!("source:{relative_path}");
    let owner_id = format!("owner:{relative_path}");
    let owner_name = source_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("module");
    let mut seen_selectors = BTreeSet::new();
    let mut items = Vec::new();
    for item in &module.syntax_facts.top_level_items {
        let Some(name) = item.name.as_deref() else {
            continue;
        };
        let kind = projection_kind(item.kind);
        let selector = format!("rust://{relative_path}#item/{kind}/{name}");
        if !seen_selectors.insert(selector.clone()) {
            continue;
        }
        items.push(json!({
            "itemId": format!("item:{kind}:{name}"),
            "ownerId": owner_id,
            "kind": kind,
            "name": name,
            "selector": selector,
        }));
    }
    let mut relations = vec![json!({
        "from": {"kind": "source", "id": source_id},
        "kind": "contains",
        "to": {"kind": "owner", "id": owner_id},
    })];
    relations.extend(items.iter().map(|item| {
        json!({
            "from": {"kind": "owner", "id": owner_id},
            "kind": "contains",
            "to": {"kind": "item", "id": item["itemId"]},
        })
    }));
    Ok(json!({
        "schemaId": "agent.semantic-protocols.semantic-language-projection",
        "schemaVersion": "1",
        "protocolId": "agent.semantic-protocols.language-projection",
        "protocolVersion": "1",
        "languageId": "rust",
        "harness": {
            "harnessId": "rust-lang-project-harness",
            "parserAbi": "syn-v2-full-v1",
            "selectorDialect": "rust",
        },
        "sources": [{
            "sourceId": source_id,
            "path": relative_path,
            "sourceKind": source_kind(&options.owner),
        }],
        "owners": [{
            "ownerId": owner_id,
            "sourceId": source_id,
            "kind": "module",
            "name": owner_name,
        }],
        "items": items,
        "relations": relations,
    }))
}

fn projection_kind(kind: &str) -> &str {
    match kind {
        "fn" => "function",
        "mod" => "module",
        other => other,
    }
}

fn validate_relative_owner(owner: &Path) -> Result<(), String> {
    if owner.is_absolute()
        || owner.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("projection owner must be a relative workspace path".to_string());
    }
    Ok(())
}

fn project_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn source_kind(owner: &Path) -> &'static str {
    if owner
        .components()
        .any(|component| component.as_os_str() == "tests")
    {
        "test"
    } else {
        "source"
    }
}

fn language_projection_usage() -> String {
    "usage: rs-harness projection <relative-owner> --workspace <root> --json".to_string()
}
