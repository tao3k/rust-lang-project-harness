//! Provider-facing `ast-patch` receipt rendering and Rust-native mutations.

use std::ffi::OsString;
use std::fs;
use std::io::{self, Read};
use std::ops::Range;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::{Value, json};

const RECEIPT_SCHEMA_ID: &str = "agent.semantic-protocols.semantic-ast-patch-receipt";
const AST_PATCH_PROTOCOL_ID: &str = "agent.semantic-protocols.ast-patch";
const SUPPORTED_OPERATIONS: &[&str] = &["replace_item"];

pub(super) fn run_ast_patch(args: impl Iterator<Item = OsString>) -> Result<ExitCode, String> {
    let args: Vec<String> = args
        .map(|arg| {
            arg.into_string()
                .map_err(|_| "ast-patch arguments must be valid UTF-8".to_string())
        })
        .collect::<Result<_, _>>()?;
    let Some(mode) = args.first().map(String::as_str) else {
        return Err(
            "usage: rs-harness ast-patch <dry-run|apply> --packet <path|-> [PROJECT_ROOT]"
                .to_string(),
        );
    };
    if mode == "--help" || mode == "-h" {
        print_help();
        return Ok(ExitCode::SUCCESS);
    }
    let mode = AstPatchMode::parse(mode)?;
    let parsed = AstPatchArgs::parse(&args[1..])?;
    let packet_text = if parsed.packet_path == "-" {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| format!("failed to read ast-patch packet from stdin: {error}"))?;
        input
    } else {
        std::fs::read_to_string(&parsed.packet_path).map_err(|error| {
            format!(
                "failed to read ast-patch packet {}: {error}",
                parsed.packet_path
            )
        })?
    };
    let project_root = parsed.project_root.unwrap_or_else(|| PathBuf::from("."));
    let receipt = render_ast_patch_receipt(mode, &packet_text, &project_root);
    let quiet_success = matches!(mode, AstPatchMode::Apply)
        && serde_json::from_str::<Value>(&receipt)
            .ok()
            .and_then(|receipt| {
                receipt
                    .get("status")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .as_deref()
            == Some("applied");
    if !quiet_success {
        println!("{receipt}");
    }
    Ok(ExitCode::SUCCESS)
}

#[derive(Clone, Copy)]
enum AstPatchMode {
    DryRun,
    Apply,
}

impl AstPatchMode {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "dry-run" => Ok(Self::DryRun),
            "apply" => Ok(Self::Apply),
            _ => Err("expected ast-patch <dry-run|apply>".to_string()),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::Apply => "apply",
        }
    }

    fn capability(self) -> &'static str {
        match self {
            Self::DryRun => "provider-ast-dry-run",
            Self::Apply => "provider-ast-apply",
        }
    }

    fn success_status(self) -> &'static str {
        match self {
            Self::DryRun => "verified",
            Self::Apply => "applied",
        }
    }

    fn mechanical_plan_kind(self) -> &'static str {
        match self {
            Self::DryRun => "provider-dry-run",
            Self::Apply => "provider-apply",
        }
    }

    fn mutation_available(self) -> bool {
        matches!(self, Self::Apply)
    }

    fn writes_files(self) -> bool {
        matches!(self, Self::Apply)
    }
}

struct AstPatchArgs {
    packet_path: String,
    project_root: Option<PathBuf>,
}

impl AstPatchArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut packet_path = None;
        let mut positionals = Vec::new();
        let mut index = 0;
        while index < args.len() {
            let arg = &args[index];
            if arg == "--packet" {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--packet requires a path or -".to_string())?;
                if value.starts_with('-') && value != "-" {
                    return Err("--packet requires a path or -".to_string());
                }
                packet_path = Some(value.clone());
                index += 2;
                continue;
            }
            if arg.starts_with('-') {
                return Err(format!("unknown ast-patch option: {arg}"));
            }
            positionals.push(PathBuf::from(arg));
            index += 1;
        }
        if positionals.len() > 1 {
            return Err("expected at most one PROJECT_ROOT argument".to_string());
        }
        Ok(Self {
            packet_path: packet_path.ok_or_else(|| "--packet requires a path or -".to_string())?,
            project_root: positionals.pop(),
        })
    }
}

#[derive(Clone)]
struct SourceRead {
    raw: String,
    path: String,
    start_line: usize,
    end_line: usize,
}

impl SourceRead {
    fn parse(value: &str) -> Result<Self, String> {
        let mut parts = value.rsplitn(3, ':');
        let end = parts
            .next()
            .ok_or_else(|| "target.read must be path:start:end".to_string())?;
        let start = parts
            .next()
            .ok_or_else(|| "target.read must be path:start:end".to_string())?;
        let path = parts
            .next()
            .ok_or_else(|| "target.read must be path:start:end".to_string())?;
        validate_project_path(path, "target.read path")?;
        let start_line = parse_line_number(start, "target.read start line")?;
        let end_line = parse_line_number(end, "target.read end line")?;
        if start_line > end_line {
            return Err("target.read start line must be before end line".to_string());
        }
        Ok(Self {
            raw: value.to_string(),
            path: path.to_string(),
            start_line,
            end_line,
        })
    }
}

fn render_ast_patch_receipt(mode: AstPatchMode, packet_text: &str, project_root: &Path) -> String {
    let packet: Value = match serde_json::from_str(packet_text) {
        Ok(packet) => packet,
        Err(error) => {
            return failure_receipt(
                mode,
                None,
                project_root,
                Vec::new(),
                "invalid-packet",
                format!("invalid JSON packet: {error}"),
                false,
            )
            .to_string();
        }
    };

    macro_rules! fail {
        ($root:expr, $verification:expr, $kind:expr, $message:expr) => {{
            return failure_receipt(
                mode,
                Some(&packet),
                $root,
                $verification,
                $kind,
                $message,
                true,
            )
            .to_string();
        }};
    }

    let mut verification = Vec::new();
    verification.push("packet-parsed");

    let target = packet.get("target").unwrap_or(&Value::Null);
    let operation = packet.get("operation").unwrap_or(&Value::Null);
    let operation_name = match operation.get("op").and_then(Value::as_str) {
        Some("replace_item") => "replace_item",
        Some(value) => fail!(
            project_root,
            verification,
            "unsupported-operation",
            format!("rust provider ast-patch supports replace_item only, got {value}")
        ),
        None => fail!(
            project_root,
            verification,
            "invalid-packet",
            "packet operation.op is required".to_string()
        ),
    };
    verification.push("operation-supported");

    let read = match target.get("read").and_then(Value::as_str) {
        Some(value) => match SourceRead::parse(value) {
            Ok(read) => read,
            Err(error) => fail!(project_root, verification, "target-read-invalid", error),
        },
        None => fail!(
            project_root,
            verification,
            "target-read-invalid",
            "packet target.read is required".to_string()
        ),
    };
    verification.push("target-read-valid");

    let snippet = match operation.get("snippet").and_then(Value::as_str) {
        Some(snippet) => snippet,
        _ => fail!(
            project_root,
            verification,
            "snippet-missing",
            format!("{operation_name} requires operation.snippet")
        ),
    };

    let replacement_item = match syn::parse_str::<syn::Item>(snippet) {
        Ok(item) => item,
        Err(error) => fail!(
            project_root,
            verification,
            "snippet-parse-error",
            format!("replacement item failed to parse: {error}")
        ),
    };
    verification.push("snippet-parsed");
    let project_root = match project_root.canonicalize() {
        Ok(path) => path,
        Err(error) => fail!(
            project_root,
            verification,
            "project-root-invalid",
            format!("failed to resolve project root: {error}")
        ),
    };
    verification.push("project-root-resolved");

    let requested_source_path = project_root.join(&read.path);
    let source_path = match requested_source_path.canonicalize() {
        Ok(path) if path.starts_with(&project_root) => path,
        Ok(_) => fail!(
            &project_root,
            verification,
            "target-outside-project",
            format!("target.read path escapes project root: {}", read.path)
        ),
        Err(error) => fail!(
            &project_root,
            verification,
            "source-read-error",
            format!("failed to resolve target.read path {}: {error}", read.path)
        ),
    };
    verification.push("target-path-resolved");

    let source = match fs::read_to_string(&source_path) {
        Ok(source) => source,
        Err(error) => fail!(
            &project_root,
            verification,
            "source-read-error",
            format!("failed to read {}: {error}", read.path)
        ),
    };

    let range = match byte_range_for_line_range(&source, read.start_line, read.end_line) {
        Ok(range) => range,
        Err(error) => fail!(&project_root, verification, "target-range-invalid", error),
    };
    verification.push("target-range-resolved");

    let existing = &source[range.clone()];
    if let Some(expected_snippet) = operation.get("expectedSnippet").and_then(Value::as_str) {
        let expected_snippet = expected_snippet.trim();
        if !expected_snippet.is_empty() && !existing.contains(expected_snippet) {
            fail!(
                &project_root,
                verification,
                "target-preimage-mismatch",
                "target preimage did not match operation.expectedSnippet".to_string()
            );
        }
        verification.push("expected-snippet-matched");
    }
    let existing_item = match syn::parse_str::<syn::Item>(existing) {
        Ok(item) => item,
        Err(error) => fail!(
            &project_root,
            verification,
            "target-item-parse-error",
            format!("selected target did not parse as Rust item: {error}")
        ),
    };
    verification.push("target-item-parsed");

    if let Err(error) = validate_item_identity(target, &existing_item, &replacement_item) {
        fail!(&project_root, verification, "target-item-mismatch", error);
    }
    verification.push("item-identity-checked");

    let replacement = normalized_replacement(snippet);
    let mut next_source = String::new();
    next_source.push_str(&source[..range.start]);
    next_source.push_str(&replacement);
    next_source.push_str(&source[range.end..]);

    if let Err(error) = crate::parser::parse_rust_source_syntax(&next_source) {
        fail!(
            &project_root,
            verification,
            "file-parse-error",
            format!("full file failed to parse after replacement: {error}")
        );
    }
    verification.push("file-reparsed");

    if mode.writes_files() {
        let formatted = match format_candidate_source(&source_path, &next_source) {
            Ok(source) => source,
            Err((failure_kind, error)) => fail!(&project_root, verification, failure_kind, error),
        };
        verification.push("rustfmt-ran");
        verification.push("formatter-output-reparsed");

        if let Err(error) = fs::write(&source_path, formatted) {
            fail!(
                &project_root,
                verification,
                "source-write-error",
                format!("failed to write {}: {error}", read.path)
            );
        }
        verification.push("file-written");
    }

    return success_receipt(
        mode,
        Some(&packet),
        &project_root,
        &read,
        operation,
        verification,
    )
    .to_string();
}

fn success_receipt(
    mode: AstPatchMode,
    packet: Option<&Value>,
    project_root: &Path,
    read: &SourceRead,
    operation: &Value,
    verification: Vec<&'static str>,
) -> Value {
    json!({
        "schemaId": RECEIPT_SCHEMA_ID,
        "schemaVersion": "1",
        "protocolId": AST_PATCH_PROTOCOL_ID,
        "protocolVersion": "1",
        "status": mode.success_status(),
        "mode": mode.as_str(),
        "capability": mode.capability(),
        "mutationAvailable": mode.mutation_available(),
        "languageId": "rust",
        "target": receipt_target(packet),
        "operation": "replace_item",
        "supportedOperations": SUPPORTED_OPERATIONS,
        "mechanicalEditPlan": {
            "kind": mode.mechanical_plan_kind(),
            "operation": "replace_item",
            "targetRead": read.raw,
            "estimatedEdits": 1,
            "maxEdits": operation_max_edits(operation),
            "safeForLargeChange": false,
            "mutationAvailable": mode.mutation_available(),
            "requiresCodexApplyPatch": false,
            "changedRanges": [read.raw],
            "notes": [
                "Rust provider parsed the replacement as syn::Item",
                "Rust provider reparsed the full file after replacement"
            ]
        },
        "verification": verification,
        "failureKind": null,
        "failures": [],
        "next": success_next_guidance(mode, project_root)
    })
}

fn failure_receipt(
    mode: AstPatchMode,
    packet: Option<&Value>,
    project_root: &Path,
    verification: Vec<&'static str>,
    failure_kind: &str,
    failure: String,
    include_supported_operations: bool,
) -> Value {
    json!({
        "schemaId": RECEIPT_SCHEMA_ID,
        "schemaVersion": "1",
        "protocolId": AST_PATCH_PROTOCOL_ID,
        "protocolVersion": "1",
        "status": "failed",
        "mode": mode.as_str(),
        "capability": mode.capability(),
        "mutationAvailable": false,
        "languageId": "rust",
        "target": receipt_target(packet),
        "operation": receipt_operation(packet),
        "supportedOperations": if include_supported_operations {
            json!(SUPPORTED_OPERATIONS)
        } else {
            json!([])
        },
        "mechanicalEditPlan": null,
        "verification": verification,
        "failureKind": failure_kind,
        "failures": [failure],
        "next": failure_next_guidance(project_root, packet)
    })
}

fn receipt_target(packet: Option<&Value>) -> Value {
    let owner_path = packet
        .and_then(|packet| packet.get("target"))
        .and_then(|target| target.get("ownerPath"))
        .and_then(Value::as_str)
        .filter(|value| validate_project_path(value, "target.ownerPath").is_ok());
    let locator = packet
        .and_then(|packet| packet.get("target"))
        .and_then(|target| target.get("locator"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());
    let read = packet
        .and_then(|packet| packet.get("target"))
        .and_then(|target| target.get("read"))
        .and_then(Value::as_str)
        .filter(|value| SourceRead::parse(value).is_ok());
    json!({
        "ownerPath": owner_path,
        "locator": locator,
        "read": read
    })
}

fn receipt_operation(packet: Option<&Value>) -> Value {
    packet
        .and_then(|packet| packet.get("operation"))
        .and_then(|operation| operation.get("op"))
        .and_then(Value::as_str)
        .filter(|operation| is_schema_operation(operation))
        .map(|operation| json!(operation))
        .unwrap_or(Value::Null)
}

fn success_next_guidance(mode: AstPatchMode, project_root: &Path) -> String {
    let project_root = project_root.display();
    match mode {
        AstPatchMode::DryRun => format!(
            "provider dry-run verified AST patch; apply with `asp rust ast-patch apply --packet semantic-ast-patch.json {project_root}`; check `asp rust check --changed {project_root}`"
        ),
        AstPatchMode::Apply => {
            format!("provider apply completed; check `asp rust check --changed {project_root}`")
        }
    }
}

fn failure_next_guidance(project_root: &Path, packet: Option<&Value>) -> String {
    let owner = packet
        .and_then(|packet| packet.get("target"))
        .and_then(|target| target.get("ownerPath"))
        .and_then(Value::as_str)
        .unwrap_or("<owner-path>");
    let read = packet
        .and_then(|packet| packet.get("target"))
        .and_then(|target| target.get("read"))
        .and_then(Value::as_str)
        .unwrap_or("<path:start:end>");
    let project_root = project_root.display();
    format!(
        "rust provider currently supports replace_item; inspect exact source with `asp rust query --from-hook direct-source-read --selector {read} --code {project_root}`; rebuild the packet with `asp ast-patch template --language rust --owner {owner} --read {read} --op replace_item --snippet '<replacement item>' > semantic-ast-patch.json`; verify with `asp rust ast-patch dry-run --packet semantic-ast-patch.json {project_root}`; apply with `asp rust ast-patch apply --packet semantic-ast-patch.json {project_root}`; check `asp rust check --changed {project_root}`"
    )
}

fn parse_line_number(value: &str, label: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{label} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{label} must be a positive integer"));
    }
    Ok(parsed)
}

fn validate_project_path(value: &str, label: &str) -> Result<(), String> {
    if value == "." {
        return Ok(());
    }
    if value.is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(format!("{label} must be project-relative"));
    }
    if value.contains(':') || value.contains('\\') {
        return Err(format!("{label} contains unsupported path characters"));
    }
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => return Err(format!("{label} must not contain dot path segments")),
        }
    }
    Ok(())
}

fn byte_range_for_line_range(
    source: &str,
    start_line: usize,
    end_line: usize,
) -> Result<Range<usize>, String> {
    if source.is_empty() {
        return Err("source file is empty".to_string());
    }
    let line_starts = std::iter::once(0)
        .chain(
            source
                .bytes()
                .enumerate()
                .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
        )
        .collect::<Vec<_>>();
    let available_lines = if source.ends_with('\n') {
        line_starts.len().saturating_sub(1)
    } else {
        line_starts.len()
    };
    if start_line > available_lines || end_line > available_lines {
        return Err(format!(
            "line range {start_line}:{end_line} is outside source with {available_lines} lines"
        ));
    }
    let start = line_starts[start_line - 1];
    let end = if end_line < line_starts.len() {
        line_starts[end_line]
    } else {
        source.len()
    };
    Ok(start..end)
}

fn normalized_replacement(snippet: &str) -> String {
    let mut replacement = snippet.trim().to_string();
    replacement.push('\n');
    replacement
}

fn validate_item_identity(
    target: &Value,
    existing: &syn::Item,
    replacement: &syn::Item,
) -> Result<(), String> {
    let existing_identity = item_identity(existing);
    let replacement_identity = item_identity(replacement);
    if let (Some(existing), Some(replacement)) = (&existing_identity, &replacement_identity) {
        if existing != replacement {
            return Err(format!(
                "replacement item identity changed from {}:{} to {}:{}",
                existing.0, existing.1, replacement.0, replacement.1
            ));
        }
    }
    if let Some(expected_name) = target.get("itemName").and_then(Value::as_str) {
        let Some((_, existing_name)) = existing_identity else {
            return Err("target.itemName was provided for an unnamed item".to_string());
        };
        if existing_name != expected_name {
            return Err(format!(
                "target.itemName {expected_name} does not match selected item {existing_name}"
            ));
        }
    }
    if let Some(expected_kind) = target.get("itemKind").and_then(Value::as_str) {
        let Some((existing_kind, _)) = item_identity(existing) else {
            return Err("target.itemKind was provided for an unnamed item".to_string());
        };
        if existing_kind != expected_kind {
            return Err(format!(
                "target.itemKind {expected_kind} does not match selected item kind {existing_kind}"
            ));
        }
    }
    Ok(())
}

fn item_identity(item: &syn::Item) -> Option<(&'static str, String)> {
    match item {
        syn::Item::Const(item) => Some(("const", item.ident.to_string())),
        syn::Item::Enum(item) => Some(("enum", item.ident.to_string())),
        syn::Item::Fn(item) => Some(("fn", item.sig.ident.to_string())),
        syn::Item::Impl(item) => type_terminal_name(&item.self_ty).map(|name| ("impl", name)),
        syn::Item::Mod(item) => Some(("mod", item.ident.to_string())),
        syn::Item::Static(item) => Some(("static", item.ident.to_string())),
        syn::Item::Struct(item) => Some(("struct", item.ident.to_string())),
        syn::Item::Trait(item) => Some(("trait", item.ident.to_string())),
        syn::Item::TraitAlias(item) => Some(("trait_alias", item.ident.to_string())),
        syn::Item::Type(item) => Some(("type", item.ident.to_string())),
        syn::Item::Union(item) => Some(("union", item.ident.to_string())),
        _ => None,
    }
}

fn type_terminal_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        syn::Type::Reference(reference) => type_terminal_name(&reference.elem),
        syn::Type::Paren(paren) => type_terminal_name(&paren.elem),
        syn::Type::Group(group) => type_terminal_name(&group.elem),
        _ => None,
    }
}

fn operation_max_edits(operation: &Value) -> u64 {
    operation
        .get("maxEdits")
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

fn format_candidate_source(
    source_path: &Path,
    next_source: &str,
) -> Result<String, (&'static str, String)> {
    let temp_path = candidate_format_path(source_path);
    if let Err(error) = fs::write(&temp_path, next_source) {
        return Err((
            "source-write-error",
            format!(
                "failed to write format candidate {}: {error}",
                temp_path.display()
            ),
        ));
    }

    let result = (|| {
        run_rustfmt(&temp_path).map_err(|error| ("rustfmt-error", error))?;
        let formatted = fs::read_to_string(&temp_path).map_err(|error| {
            (
                "source-read-error",
                format!(
                    "failed to read formatted candidate {}: {error}",
                    temp_path.display()
                ),
            )
        })?;
        crate::parser::parse_rust_source_syntax(&formatted).map_err(|error| {
            (
                "formatter-parse-error",
                format!("rustfmt output failed to parse: {error}"),
            )
        })?;
        Ok(formatted)
    })();

    let _ = fs::remove_file(&temp_path);
    result
}

fn candidate_format_path(source_path: &Path) -> PathBuf {
    let file_name = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("ast-patch.rs");
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let temp_name = format!(".{file_name}.{}.{}.tmp.rs", std::process::id(), nonce);
    source_path.with_file_name(temp_name)
}

fn run_rustfmt(path: &Path) -> Result<(), String> {
    let output = Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .arg(path)
        .output()
        .map_err(|error| format!("failed to run rustfmt: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(format!(
        "rustfmt failed with status {}; stdout: {}; stderr: {}",
        output.status,
        stdout.trim(),
        stderr.trim()
    ))
}

fn is_schema_operation(value: &str) -> bool {
    matches!(
        value,
        "append_to_block"
            | "insert_before_statement"
            | "insert_after_statement"
            | "replace_statement"
            | "replace_expression"
            | "replace_call_arg"
            | "insert_import"
            | "remove_import"
            | "remove_statement"
            | "remove_item"
            | "replace_item"
    )
}

fn print_help() {
    println!(
        "Usage: asp rust ast-patch <dry-run|apply> --packet <semantic-ast-patch.json|-> [project-root]"
    );
    println!();
    println!("Emits a provider AST patch receipt. Rust native apply supports replace_item.");
}
