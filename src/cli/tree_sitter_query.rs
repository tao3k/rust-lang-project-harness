use std::ffi::OsString;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::ExitCode;

use super::query::print_query_help;
use super::tree_sitter_query_locator::{
    SyntaxQuerySelector, parse_syntax_query_selector, syntax_line_locator,
};
use super::tree_sitter_query_packet::{
    SyntaxQueryRow, syntax_query_matches_json, syntax_query_native_fact_refs,
};
use super::tree_sitter_query_projection::{
    SUPPORTED_TREE_SITTER_QUERY_NODES, project_tree_sitter_query,
};

struct RustTreeSitterCatalog {
    id: &'static str,
    path: &'static str,
    source: &'static str,
}

const RUST_TREE_SITTER_GRAMMAR_ID: &str = "tree-sitter-rust";
const RUST_TREE_SITTER_GRAMMAR_PROFILE_VERSION: &str = "2026-06-04.v1";
const RUST_TREE_SITTER_GRAMMAR_PROFILE_PATH: &str =
    "tree-sitter/tree-sitter-rust/grammar-profile.json";
const RUST_TREE_SITTER_GRAMMAR_PROFILE_SOURCE: &str =
    include_str!("../../tree-sitter/tree-sitter-rust/grammar-profile.json");
pub(super) fn run_tree_sitter_query_catalog(args: &[OsString]) -> Result<Option<ExitCode>, String> {
    if !args
        .iter()
        .any(|arg| arg == "--catalog" || arg == "--treesitter-query")
    {
        return Ok(None);
    }

    let mut catalog_id = None::<String>;
    let mut tree_sitter_query = None::<String>;
    let mut selector = None::<SyntaxQuerySelector>;
    let mut terms = Vec::<String>::new();
    let mut json_output = false;
    let mut code_output = false;
    let mut positionals = Vec::<PathBuf>::new();
    let mut pending_option = None::<String>;

    for arg in args {
        let value = arg
            .to_str()
            .ok_or_else(|| format!("query argument is not valid UTF-8: {arg:?}"))?;
        if let Some(option) = pending_option.take() {
            match option.as_str() {
                "--catalog" => {
                    catalog_id = Some(value.to_string());
                }
                "--treesitter-query" => {
                    tree_sitter_query = Some(value.to_string());
                }
                "--term" => {
                    terms.push(value.to_string());
                }
                "--selector" => {
                    selector = Some(parse_syntax_query_selector(value)?);
                }
                _ => unreachable!("unsupported pending query catalog option: {option}"),
            }
            continue;
        }

        match value {
            "--catalog" => {
                pending_option = Some(value.to_string());
            }
            "--treesitter-query" => {
                pending_option = Some(value.to_string());
            }
            "--term" => {
                pending_option = Some(value.to_string());
            }
            "--selector" => {
                pending_option = Some(value.to_string());
            }
            "--json" => {
                json_output = true;
            }
            "--code" => {
                code_output = true;
            }
            "--help" | "-h" => {
                print_query_help();
                return Ok(Some(ExitCode::SUCCESS));
            }
            option if option.starts_with('-') => {
                return Err(format!("unsupported query catalog option: {option}"));
            }
            other => {
                positionals.push(PathBuf::from(other));
            }
        }
    }

    if let Some(option) = pending_option {
        return Err(format!("missing value for query catalog option {option}"));
    }
    if positionals.len() > 1 {
        return Err("query catalog accepts at most one project root".to_string());
    }
    if catalog_id.is_some() && tree_sitter_query.is_some() {
        return Err("query accepts only one of --catalog or --treesitter-query".to_string());
    }

    let project_root = positionals
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."));

    let (
        input,
        input_form,
        catalog_id,
        catalog_path,
        catalog_source,
        catalog_canonical,
        catalog_embedded,
    ) = if let Some(catalog_id) = catalog_id {
        let catalog = rust_tree_sitter_catalog(&catalog_id)
            .ok_or_else(|| format!("unknown Rust tree-sitter query catalog: {catalog_id}"))?;
        (
            catalog.id.to_string(),
            "catalog-id",
            Some(catalog.id.to_string()),
            Some(catalog.path.to_string()),
            catalog.source.to_string(),
            true,
            true,
        )
    } else {
        let query = tree_sitter_query
            .ok_or_else(|| "missing --catalog or --treesitter-query value".to_string())?;
        let query = query.trim().to_string();
        if query.is_empty() {
            return Err("query --treesitter-query value cannot be empty".to_string());
        }
        (
            query.clone(),
            "s-expression",
            None,
            None,
            query,
            false,
            false,
        )
    };

    let mut captures = catalog_source
        .split(|character: char| {
            character.is_whitespace() || matches!(character, '(' | ')' | '[' | ']' | '{' | '}')
        })
        .filter_map(|token| token.strip_prefix('@'))
        .filter(|capture| !capture.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    captures.sort();
    captures.dedup();

    let profile_source = RUST_TREE_SITTER_GRAMMAR_PROFILE_SOURCE;
    let mut catalog_hasher = std::collections::hash_map::DefaultHasher::new();
    catalog_source.hash(&mut catalog_hasher);
    let catalog_hash = catalog_hasher.finish();
    let catalog_fingerprint = format!("rust-default:{catalog_hash:016x}");
    let mut profile_hasher = std::collections::hash_map::DefaultHasher::new();
    profile_source.hash(&mut profile_hasher);
    let grammar_profile_fingerprint = format!("rust-default:{:016x}", profile_hasher.finish());
    let query_identity = catalog_id.as_deref().unwrap_or("inline");
    let artifact_stem = catalog_id
        .clone()
        .unwrap_or_else(|| format!("inline-{catalog_hash:016x}"));
    let request_fingerprint = format!(
        "semantic-tree-sitter-query.v1:rust:{RUST_TREE_SITTER_GRAMMAR_ID}:{query_identity}:{catalog_fingerprint}:{grammar_profile_fingerprint}",
    );
    let projection = project_tree_sitter_query(
        &project_root,
        &catalog_source,
        &captures,
        &terms,
        selector.as_ref(),
    )?;

    if json_output {
        let mut query_fields = serde_json::json!({
            "captures": captures,
            "catalogCanonical": catalog_canonical,
            "catalogEmbedded": catalog_embedded,
            "compilerBoundary": "asp-tree-sitter-runtime",
            "providerRuntimeCompiled": false,
            "codeOutput": code_output,
            "terms": terms
        });
        if let Some(selector) = selector.as_ref() {
            query_fields["selector"] = serde_json::json!(selector.display());
        }
        let mut query = serde_json::json!({
            "input": input,
            "inputForm": input_form,
            "dialect": "tree-sitter-query",
            "grammarProfilePath": RUST_TREE_SITTER_GRAMMAR_PROFILE_PATH,
            "compiledSource": catalog_source,
            "fields": query_fields
        });
        if let Some(catalog_id) = catalog_id.as_deref() {
            query["catalogId"] = serde_json::json!(catalog_id);
        }
        if let Some(catalog_path) = catalog_path.as_deref() {
            query["catalogPath"] = serde_json::json!(catalog_path);
        }
        let matches = syntax_query_matches_json(&projection.rows);
        let native_fact_refs = syntax_query_native_fact_refs(&projection.rows);

        let packet = serde_json::json!({
            "schemaId": "agent.semantic-protocols.semantic-tree-sitter-query",
            "schemaVersion": "1",
            "protocolId": "agent.semantic-protocols.semantic-language",
            "protocolVersion": "1",
            "languageId": "rust",
            "providerId": "rs-harness",
            "method": "query",
            "projectRoot": project_root.display().to_string(),
            "grammarId": RUST_TREE_SITTER_GRAMMAR_ID,
            "grammarProfileVersion": RUST_TREE_SITTER_GRAMMAR_PROFILE_VERSION,
            "sourceAuthority": "native-parser-adapter",
            "adapterMode": "native-projection",
            "compatibilityLevel": "native-only",
            "query": query,
            "matches": matches,
            "nativeFactRefs": native_fact_refs,
            "truncated": projection.truncated,
            "cache": {
                "cacheStatus": "miss",
                "requestFingerprint": request_fingerprint,
                "generationId": format!("rust-tree-sitter-query:{artifact_stem}:{RUST_TREE_SITTER_GRAMMAR_PROFILE_VERSION}"),
                "artifactId": format!("semantic-tree-sitter-query/{artifact_stem}.json"),
                "artifactKind": "semantic-tree-sitter-query",
                "catalogFingerprint": catalog_fingerprint,
                "grammarProfileFingerprint": grammar_profile_fingerprint,
                "rawSourceStored": false
            }
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&packet).map_err(|error| format!(
                "failed to serialize semantic tree-sitter query packet: {error}"
            ))?
        );
    } else {
        if code_output {
            print_tree_sitter_query_code(&projection.rows);
            return Ok(Some(ExitCode::SUCCESS));
        }
        if projection.rows.is_empty() {
            let term_field = if terms.is_empty() {
                String::new()
            } else {
                format!(" terms={}", terms.join(","))
            };
            let captures_display = if captures.is_empty() {
                "none".to_string()
            } else {
                captures.join(",")
            };
            println!(
                "|syntax-query inputForm={} input={} grammar={} grammarProfile={} dialect=tree-sitter-query mode=native-parser-projection matchStatus={} match={} rows={} truncated={} captureCount={} captures={}{} catalogCanonical={} catalogEmbedded={} sourceAuthority=native-parser compilerBoundary=asp-tree-sitter-runtime providerRuntimeCompiled=false",
                input_form,
                query_identity,
                RUST_TREE_SITTER_GRAMMAR_ID,
                RUST_TREE_SITTER_GRAMMAR_PROFILE_VERSION,
                projection.match_status(),
                projection.total_matches,
                projection.rows.len(),
                projection.truncated,
                captures.len(),
                captures_display,
                term_field,
                catalog_canonical,
                catalog_embedded
            );
            if !projection.unsupported_nodes.is_empty() {
                println!(
                    "|syntax-query-unsupported nodes={} supported={}",
                    projection.unsupported_nodes.join(","),
                    SUPPORTED_TREE_SITTER_QUERY_NODES.join(",")
                );
            }
        }
        if !projection.rows.is_empty() {
            print_tree_sitter_query_locators(&projection.rows);
        }
        if projection.truncated {
            println!(
                "truncated rows={} total={} next=narrow-query-or-combine-with-owner",
                projection.rows.len(),
                projection.total_matches
            );
        }
    }

    Ok(Some(ExitCode::SUCCESS))
}

fn print_tree_sitter_query_code(rows: &[SyntaxQueryRow]) {
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            println!();
        }
        println!("{}", row.item_code);
    }
}

fn print_tree_sitter_query_locators(rows: &[SyntaxQueryRow]) {
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            println!();
        }
        println!(
            "{}",
            syntax_line_locator(&row.path, row.start_line, row.end_line)
        );
        println!("{}", row.capture_text);
    }
}

fn rust_tree_sitter_catalog(catalog_id: &str) -> Option<RustTreeSitterCatalog> {
    match catalog_id {
        "declarations" => Some(RustTreeSitterCatalog {
            id: "declarations",
            path: "tree-sitter/tree-sitter-rust/queries/declarations.scm",
            source: include_str!("../../tree-sitter/tree-sitter-rust/queries/declarations.scm"),
        }),
        "imports" => Some(RustTreeSitterCatalog {
            id: "imports",
            path: "tree-sitter/tree-sitter-rust/queries/imports.scm",
            source: include_str!("../../tree-sitter/tree-sitter-rust/queries/imports.scm"),
        }),
        "calls" => Some(RustTreeSitterCatalog {
            id: "calls",
            path: "tree-sitter/tree-sitter-rust/queries/calls.scm",
            source: include_str!("../../tree-sitter/tree-sitter-rust/queries/calls.scm"),
        }),
        "macros" => Some(RustTreeSitterCatalog {
            id: "macros",
            path: "tree-sitter/tree-sitter-rust/queries/macros.scm",
            source: include_str!("../../tree-sitter/tree-sitter-rust/queries/macros.scm"),
        }),
        "cfg" => Some(RustTreeSitterCatalog {
            id: "cfg",
            path: "tree-sitter/tree-sitter-rust/queries/cfg.scm",
            source: include_str!("../../tree-sitter/tree-sitter-rust/queries/cfg.scm"),
        }),
        _ => None,
    }
}
