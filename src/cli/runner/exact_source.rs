use std::process::ExitCode;

use crate::cli::query::ExactSourceQuery;

pub(super) fn run_exact_source_query(options: ExactSourceQuery) -> Result<ExitCode, String> {
    let source_snapshot_envelope = options.source_snapshot_envelope.as_ref().ok_or_else(|| {
        "exact source query state=source-unavailable reasonKind=wrapper-snapshot-required"
            .to_string()
    })?;
    let selector = ExactSelector::parse(&options.selector)
        .map_err(|error| format!("exact source query state=invalid-selector {error}"))?;
    let pinned = PinnedWorkspace::load(source_snapshot_envelope).map_err(|error| {
        format!(
            "exact source query state=source-unavailable reasonKind=snapshot-envelope-invalid {error}"
        )
    })?;
    let requested_owner_exists = pinned.sources.contains_key(&selector.owner_path);
    let (resolved, state) = if let Some(resolved) = resolve_live_item(&pinned, &selector)? {
        (resolved, "live-hit")
    } else {
        match relocate_live_item(&pinned, &selector)? {
            RelocationOutcome::Resolved(resolved) => (resolved, "live-relocated"),
            RelocationOutcome::Ambiguous(candidates) => {
                return exact_source_failure(
                    &selector,
                    &pinned,
                    "ambiguous",
                    "multiple-snapshot-items",
                    candidates,
                    Vec::new(),
                );
            }
            RelocationOutcome::KindMismatch(actual_kinds) => {
                return exact_source_failure(
                    &selector,
                    &pinned,
                    "kind-mismatch",
                    "snapshot-item-kind-mismatch",
                    Vec::new(),
                    actual_kinds,
                );
            }
            RelocationOutcome::Missing => {
                let state = if requested_owner_exists {
                    "item-missing"
                } else {
                    "owner-missing"
                };
                let reason_kind = if requested_owner_exists {
                    "item-not-in-live-owner"
                } else {
                    "owner-not-in-snapshot"
                };
                return exact_source_failure(
                    &selector,
                    &pinned,
                    state,
                    reason_kind,
                    Vec::new(),
                    Vec::new(),
                );
            }
        }
    };
    let code = resolved.code.trim_end_matches('\n').to_string();

    if options.json {
        let provider_id = options
            .provider_id
            .as_deref()
            .ok_or_else(|| "exact source projection requires --asp-provider-id".to_string())?;
        if provider_id != pinned.provider_id {
            return Err(format!(
                "exact source projection provider mismatch: expected={} actual={provider_id}",
                pinned.provider_id
            ));
        }
        let parser_identity_digest =
            agent_semantic_content_identity::exact_selector_merkle::parse_content_digest_v1(
                options.parser_identity_digest.as_deref().ok_or_else(|| {
                    "exact source projection requires --asp-parser-identity-digest".to_string()
                })?,
            )?;
        let query_pack_digest =
            agent_semantic_content_identity::exact_selector_merkle::parse_content_digest_v1(
                options.query_pack_digest.as_deref().ok_or_else(|| {
                    "exact source projection requires --asp-query-pack-digest".to_string()
                })?,
            )?;
        let source = pinned.sources.get(&resolved.owner_path).ok_or_else(|| {
            format!(
                "exact source projection resolved an owner outside the pinned snapshot: {}",
                resolved.owner_path
            )
        })?;
        let normalized_parser_facts = serde_json::to_vec(&serde_json::json!({
            "itemKind": resolved.item_kind,
            "itemName": resolved.item_name,
            "ownerPath": resolved.owner_path,
            "resolvedSelector": resolved.selector,
            "resolutionState": state,
        }))
        .map_err(|error| format!("serialize exact source parser facts: {error}"))?;
        let packet = agent_semantic_content_identity::exact_selector_projection_packet::build_exact_selector_projection_packet_v1(
            "rust",
            provider_id,
            &parser_identity_digest,
            &query_pack_digest,
            &resolved.owner_path,
            &options.selector,
            agent_semantic_content_identity::exact_selector_merkle::ExactProjectionModeV1::Code,
            source.source.as_bytes(),
            &normalized_parser_facts,
            code.as_bytes(),
        );
        println!(
            "{}",
            serde_json::to_string(&packet)
                .map_err(|error| format!("serialize exact source projection packet: {error}"))?
        );
    } else if options.names_only {
        println!("{}", resolved.item_name);
    } else if options.code {
        println!("{code}");
    } else {
        println!("{}", resolved.selector);
    }

    Ok(ExitCode::SUCCESS)
}

fn parse_exact_source_selector(selector: &str) -> Result<(&str, &str, &str), String> {
    let selector = selector
        .strip_prefix("rust://")
        .ok_or_else(|| format!("exact source selector `{selector}` must start with rust://"))?;
    let (owner_path, item_selector) = selector
        .split_once("#item/")
        .ok_or_else(|| format!("exact source selector `{selector}` must include #item/"))?;
    let (item_kind, item_name) = item_selector.split_once('/').ok_or_else(|| {
        format!("exact source selector `{selector}` must include item kind and name")
    })?;
    if owner_path.is_empty() || item_kind.is_empty() || item_name.is_empty() {
        return Err(format!("exact source selector `{selector}` is incomplete"));
    }
    Ok((owner_path, item_kind, item_name))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExactSelector {
    owner_path: String,
    item_kind: String,
    item_name: String,
}

impl ExactSelector {
    fn parse(selector: &str) -> Result<Self, String> {
        let (owner_path, item_kind, item_name) = parse_exact_source_selector(selector)?;
        let owner = std::path::Path::new(owner_path);
        if owner.is_absolute()
            || owner.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir | std::path::Component::RootDir
                )
            })
        {
            return Err(format!(
                "exact source selector `{selector}` escapes workspace"
            ));
        }
        let owner_path = owner_path.replace('\\', "/");
        if owner_path
            .split('/')
            .any(|segment| segment.is_empty() || segment == ".")
        {
            return Err(format!(
                "exact source selector `{selector}` has a non-canonical owner path"
            ));
        }
        Ok(Self {
            owner_path,
            item_kind: normalize_exact_item_kind(item_kind).to_string(),
            item_name: item_name.to_string(),
        })
    }
}

#[derive(Clone, Debug)]
struct ParseArtifactItem {
    kind: String,
    name: String,
    qualified_name: Option<String>,
    start_line: usize,
    end_line: usize,
}

#[derive(Clone, Debug)]
struct PinnedSource {
    source: String,
    blob_digest: String,
    parser_artifact_digest: Option<String>,
    parse_error: Option<String>,
    items: Vec<ParseArtifactItem>,
}

#[derive(Clone, Debug)]
struct PinnedWorkspace {
    provider_id: String,
    root_digest: String,
    sources: std::collections::BTreeMap<String, PinnedSource>,
}

fn snapshot_digest_is_valid(digest: &str) -> bool {
    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl PinnedWorkspace {
    fn load(envelope_path: &std::path::Path) -> Result<Self, String> {
        let envelope = std::fs::read(envelope_path).map_err(|error| {
            format!(
                "failed to read source snapshot envelope {}: {error}",
                envelope_path.display()
            )
        })?;
        let envelope: ExactSourceSnapshotEnvelopeV1 =
            serde_json::from_slice(&envelope).map_err(|error| {
                format!(
                    "failed to decode source snapshot envelope {}: {error}",
                    envelope_path.display()
                )
            })?;
        if envelope.schema_id != "asp.exact-source-snapshot-envelope.v1"
            || envelope.schema_version != "1"
        {
            return Err(format!(
                "unsupported source snapshot envelope schemaId={} schemaVersion={}",
                envelope.schema_id, envelope.schema_version
            ));
        }
        if envelope.provider_id.is_empty()
            || envelope.source_snapshot.schema_id != "asp.source-snapshot.v1"
            || envelope.source_snapshot.root_digest.len() != 64
            || envelope
                .source_snapshot
                .root_digest
                .chars()
                .any(|character| !character.is_ascii_hexdigit())
            || envelope.source_snapshot.algorithm != "blake3-merkle-v1"
            || envelope.source_snapshot.provider_digest.len() != 64
            || envelope
                .source_snapshot
                .provider_digest
                .chars()
                .any(|character| !character.is_ascii_hexdigit())
        {
            return Err(
                "source snapshot envelope lacks complete v1 authority evidence".to_string(),
            );
        }
        let mut sources = std::collections::BTreeMap::new();
        for owner in envelope.owners {
            let relative_path = normalize_snapshot_owner_path(&owner.path)?;
            if !snapshot_digest_is_valid(owner.snapshot_leaf_digest.as_str())
                || !snapshot_digest_is_valid(owner.blob_digest.as_str())
            {
                return Err(format!(
                    "source snapshot owner {} has an invalid blob digest",
                    owner.path
                ));
            }
            let cas_path = normalize_snapshot_owner_path(&owner.cas_path)?;
            let source_path = envelope.cas_root.join(cas_path);
            let bytes = std::fs::read(&source_path).map_err(|error| {
                format!(
                    "failed to read pinned source blob {} for owner {}: {error}",
                    source_path.display(),
                    relative_path
                )
            })?;
            let source = String::from_utf8(bytes).map_err(|error| {
                format!(
                    "failed to decode pinned source blob for owner {} as UTF-8: {error}",
                    relative_path
                )
            })?;
            let mut items = Vec::new();
            let parse_error = match crate::parser::parse_rust_source_syntax(&source) {
                Ok(syntax) => {
                    collect_parse_artifact_items(&source, &syntax.items, &mut items);
                    None
                }
                Err(error) => Some(error.to_string()),
            };
            sources.insert(
                relative_path,
                PinnedSource {
                    source,
                    blob_digest: owner.blob_digest,
                    parser_artifact_digest: owner.parser_artifact_digest,
                    parse_error,
                    items,
                },
            );
        }
        if envelope.source_snapshot.leaf_count < sources.len() {
            return Err(format!(
                "source snapshot leaf count is smaller than provider owner count: leafCount={} ownerCount={}",
                envelope.source_snapshot.leaf_count,
                sources.len()
            ));
        }
        Ok(Self {
            provider_id: envelope.provider_id,
            root_digest: envelope.source_snapshot.root_digest,
            sources,
        })
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExactSourceSnapshotEnvelopeV1 {
    schema_id: String,
    schema_version: String,
    provider_id: String,
    source_snapshot: ExactSourceSnapshotEvidenceV1,
    cas_root: std::path::PathBuf,
    owners: Vec<ExactSourceSnapshotOwnerV1>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExactSourceSnapshotEvidenceV1 {
    schema_id: String,
    algorithm: String,
    root_digest: String,
    leaf_count: usize,
    provider_digest: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExactSourceSnapshotOwnerV1 {
    path: String,
    snapshot_leaf_digest: String,
    blob_digest: String,
    cas_path: String,
    #[serde(default)]
    parser_artifact_digest: Option<String>,
}

fn normalize_snapshot_owner_path(path: &str) -> Result<String, String> {
    let path = std::path::Path::new(path);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        })
    {
        return Err(format!(
            "source snapshot owner path escapes workspace: {}",
            path.display()
        ));
    }
    let normalized = path.to_string_lossy().replace('\\', "/");
    if normalized.is_empty()
        || normalized
            .split('/')
            .any(|segment| segment.is_empty() || segment == ".")
    {
        return Err(format!(
            "source snapshot owner path is not canonical: {normalized}"
        ));
    }
    Ok(normalized)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedExactItem {
    selector: String,
    owner_path: String,
    item_kind: String,
    item_name: String,
    code: String,
    owner_blob_digest: String,
    parser_artifact_digest: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RelocationOutcome {
    Resolved(ResolvedExactItem),
    Ambiguous(Vec<String>),
    KindMismatch(Vec<String>),
    Missing,
}

fn resolve_live_item(
    workspace: &PinnedWorkspace,
    selector: &ExactSelector,
) -> Result<Option<ResolvedExactItem>, String> {
    let Some(source) = workspace.sources.get(&selector.owner_path) else {
        return Ok(None);
    };
    if let Some(error) = source.parse_error.as_deref() {
        return Err(format!(
            "exact source query state=parser-failed rootDigest={} ownerPath={} error={error}",
            workspace.root_digest, selector.owner_path
        ));
    }
    let matches = source
        .items
        .iter()
        .filter(|item| exact_item_name_matches(item, &selector.item_name))
        .filter(|item| exact_item_kind_matches(&item.kind, &selector.item_kind))
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(format!(
            "exact source query state=ambiguous rootDigest={} ownerPath={} itemKind={} itemName={} matches={}",
            workspace.root_digest,
            selector.owner_path,
            selector.item_kind,
            selector.item_name,
            matches.len()
        ));
    }
    Ok(matches
        .into_iter()
        .next()
        .map(|item| resolved_exact_item(&selector.owner_path, source, item)))
}

fn relocate_live_item(
    workspace: &PinnedWorkspace,
    selector: &ExactSelector,
) -> Result<RelocationOutcome, String> {
    let mut resolved = Vec::new();
    let mut actual_kinds = std::collections::BTreeSet::new();
    for (owner_path, source) in &workspace.sources {
        if source.parse_error.is_some() {
            continue;
        }
        for item in &source.items {
            if !exact_item_name_matches(item, &selector.item_name) {
                continue;
            }
            if exact_item_kind_matches(&item.kind, &selector.item_kind) {
                resolved.push(resolved_exact_item(owner_path, source, item));
            } else {
                actual_kinds.insert(item.kind.clone());
            }
        }
    }
    resolved.sort_by(|left, right| left.selector.cmp(&right.selector));
    match resolved.len() {
        0 if !actual_kinds.is_empty() => Ok(RelocationOutcome::KindMismatch(
            actual_kinds.into_iter().collect(),
        )),
        0 => Ok(RelocationOutcome::Missing),
        1 => Ok(RelocationOutcome::Resolved(
            resolved.pop().expect("one relocation candidate"),
        )),
        _ => Ok(RelocationOutcome::Ambiguous(
            resolved.into_iter().map(|item| item.selector).collect(),
        )),
    }
}

fn collect_parse_artifact_items(
    source: &str,
    items: &[syn::Item],
    output: &mut Vec<ParseArtifactItem>,
) {
    for item in items {
        match item {
            syn::Item::Const(item) => {
                push_parse_artifact_item("const", item.ident.to_string(), None, item, output)
            }
            syn::Item::Enum(item) => {
                push_parse_artifact_item("enum", item.ident.to_string(), None, item, output)
            }
            syn::Item::Fn(item) => {
                push_parse_artifact_item("function", item.sig.ident.to_string(), None, item, output)
            }
            syn::Item::Macro(item) => collect_macro_parse_artifact_item(item, output),
            syn::Item::Mod(item) => collect_module_parse_artifact_items(source, item, output),
            syn::Item::Static(item) => {
                push_parse_artifact_item("static", item.ident.to_string(), None, item, output)
            }
            syn::Item::Struct(item) => {
                push_parse_artifact_item("struct", item.ident.to_string(), None, item, output)
            }
            syn::Item::Trait(item) => {
                push_parse_artifact_item("trait", item.ident.to_string(), None, item, output)
            }
            syn::Item::TraitAlias(item) => {
                push_parse_artifact_item("trait-alias", item.ident.to_string(), None, item, output)
            }
            syn::Item::Type(item) => {
                push_parse_artifact_item("type", item.ident.to_string(), None, item, output)
            }
            syn::Item::Union(item) => {
                push_parse_artifact_item("union", item.ident.to_string(), None, item, output)
            }
            syn::Item::Impl(item) => collect_impl_parse_artifact_items(item, output),
            syn::Item::Use(item) if !matches!(item.vis, syn::Visibility::Inherited) => {
                let span = syn::spanned::Spanned::span(item);
                collect_reexport_items(
                    &item.tree,
                    span.start().line.max(1),
                    span.end().line.max(span.start().line.max(1)),
                    output,
                );
            }
            _ => {}
        }
    }
}

fn collect_macro_parse_artifact_item(item: &syn::ItemMacro, output: &mut Vec<ParseArtifactItem>) {
    let Some(ident) = item.ident.as_ref() else {
        return;
    };
    push_parse_artifact_item("macro", ident.to_string(), None, item, output);
}

fn collect_module_parse_artifact_items(
    source: &str,
    item: &syn::ItemMod,
    output: &mut Vec<ParseArtifactItem>,
) {
    push_parse_artifact_item("module", item.ident.to_string(), None, item, output);
    let Some((_, nested)) = item.content.as_ref() else {
        return;
    };
    collect_parse_artifact_items(source, nested, output);
}

fn collect_impl_parse_artifact_items(item: &syn::ItemImpl, output: &mut Vec<ParseArtifactItem>) {
    let impl_owner = quote::ToTokens::to_token_stream(item.self_ty.as_ref())
        .to_string()
        .replace(' ', "");
    let methods = item.items.iter().filter_map(|item| match item {
        syn::ImplItem::Fn(method) => Some(method),
        _ => None,
    });
    for method in methods {
        push_parse_artifact_item(
            "method",
            method.sig.ident.to_string(),
            Some(format!("{impl_owner}::{}", method.sig.ident)),
            method,
            output,
        );
    }
}

fn collect_reexport_items(
    tree: &syn::UseTree,
    start_line: usize,
    end_line: usize,
    output: &mut Vec<ParseArtifactItem>,
) {
    match tree {
        syn::UseTree::Name(name) => output.push(ParseArtifactItem {
            kind: "reexport".to_string(),
            name: name.ident.to_string(),
            qualified_name: None,
            start_line,
            end_line,
        }),
        syn::UseTree::Rename(rename) => output.push(ParseArtifactItem {
            kind: "reexport".to_string(),
            name: rename.rename.to_string(),
            qualified_name: None,
            start_line,
            end_line,
        }),
        syn::UseTree::Path(path) => {
            collect_reexport_items(&path.tree, start_line, end_line, output)
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_reexport_items(item, start_line, end_line, output);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn push_parse_artifact_item<T: syn::spanned::Spanned>(
    kind: &str,
    name: String,
    qualified_name: Option<String>,
    item: &T,
    output: &mut Vec<ParseArtifactItem>,
) {
    let span = item.span();
    let start_line = span.start().line.max(1);
    output.push(ParseArtifactItem {
        kind: kind.to_string(),
        name,
        qualified_name,
        start_line,
        end_line: span.end().line.max(start_line),
    });
}

fn resolved_exact_item(
    owner_path: &str,
    source: &PinnedSource,
    item: &ParseArtifactItem,
) -> ResolvedExactItem {
    let item_name = item
        .qualified_name
        .as_deref()
        .unwrap_or(item.name.as_str())
        .to_string();
    ResolvedExactItem {
        selector: format!("rust://{owner_path}#item/{}/{}", item.kind, item_name),
        owner_path: owner_path.to_string(),
        item_kind: item.kind.clone(),
        item_name,
        code: source_line_window(&source.source, item.start_line, item.end_line),
        owner_blob_digest: source.blob_digest.clone(),
        parser_artifact_digest: source.parser_artifact_digest.clone(),
    }
}

fn source_line_window(source: &str, start_line: usize, end_line: usize) -> String {
    source
        .lines()
        .skip(start_line.saturating_sub(1))
        .take(end_line.saturating_sub(start_line).saturating_add(1))
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_exact_item_kind(kind: &str) -> &str {
    match kind {
        "fn" => "function",
        "mod" => "module",
        "use" | "import" => "reexport",
        other => other,
    }
}

fn exact_item_kind_matches(actual: &str, requested: &str) -> bool {
    normalize_exact_item_kind(actual) == normalize_exact_item_kind(requested)
}

fn exact_item_name_matches(item: &ParseArtifactItem, requested: &str) -> bool {
    item.name == requested || item.qualified_name.as_deref() == Some(requested)
}

fn exact_source_failure(
    selector: &ExactSelector,
    pinned: &PinnedWorkspace,
    state: &str,
    reason_kind: &str,
    candidates: Vec<String>,
    actual_kinds: Vec<String>,
) -> Result<ExitCode, String> {
    Err(format!(
        "exact source query state={state} reasonKind={reason_kind} rootDigest={} ownerPath={} itemKind={} itemName={} candidates={} actualKinds={}",
        pinned.root_digest,
        selector.owner_path,
        selector.item_kind,
        selector.item_name,
        candidates.join(","),
        actual_kinds.join(",")
    ))
}
#[cfg(test)]
#[path = "../../../tests/unit/cli/runner/exact_source.rs"]
mod tests;
