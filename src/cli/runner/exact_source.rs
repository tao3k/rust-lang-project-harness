use std::process::ExitCode;

use crate::cli::query::ExactSourceQuery;
use crate::cli::{QuerySourceVersion, render_query_local_item_code};

pub(super) fn run_exact_source_query(options: ExactSourceQuery) -> Result<ExitCode, String> {
    if let Some(source_overlay) = options.source_overlay {
        return Err(format!(
            "exact source query source overlay is not supported by the local runner yet: {}",
            source_overlay.display()
        ));
    }
    let (owner_path, item_kind, item_name) = parse_exact_source_selector(&options.selector)?;
    let code = render_query_local_item_code(
        &options.workspace_root,
        owner_path,
        item_name,
        QuerySourceVersion::Worktree,
    )?
    .ok_or_else(|| {
        format!(
            "exact source selector `{}` did not resolve under workspace `{}`",
            options.selector,
            options.workspace_root.display()
        )
    })?
    .trim_end_matches('\n')
    .to_string();

    if options.json {
        let root_digest = format!("worktree:{}", options.workspace_root.display());
        let packet = serde_json::json!({
            "schemaId": "asp.exact-source-query-result.v1",
            "schemaVersion": "1",
            "selector": options.selector,
            "resolvedOwnerPath": owner_path,
            "itemKind": item_kind,
            "itemName": item_name,
            "code": code,
            "sourceSnapshot": {
                "rootDigest": root_digest,
            },
            "resolutionEvidence": {
                "state": "live-hit",
                "authority": "live-parser",
                "parserArtifactDigest": root_digest,
                "snapshotRoot": root_digest,
            },
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&packet)
                .map_err(|error| format!("serialize exact source query packet: {error}"))?
        );
    } else if options.names_only {
        println!("{item_name}");
    } else if options.code {
        println!("{code}");
    } else {
        println!("{owner_path}#{item_kind}/{item_name}");
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
