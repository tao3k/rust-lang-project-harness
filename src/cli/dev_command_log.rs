use std::collections::BTreeSet;
use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SCHEMA_ID: &str = "agent.semantic-protocols.dev-command-log";
const SCHEMA_VERSION: &str = "1";
const PROTOCOL_ID: &str = "agent.semantic-protocols.semantic-language";
const PROTOCOL_VERSION: &str = "1";

const SECRET_FLAGS: &[&str] = &["--api-key", "--apikey", "--password", "--secret", "--token"];

const PROJECT_ANCHORS: &[&str] = &[
    "Cargo.toml",
    "package.json",
    "pnpm-lock.yaml",
    "pyproject.toml",
    "Project.toml",
    ".git",
];

const VALUE_OPTIONS: &[&str] = &[
    "--from-hook",
    "--package",
    "--query",
    "--query-set",
    "--selector",
    "--term",
    "--view",
];

const SEARCH_PIPES: &[&str] = &[
    "dependency",
    "docs",
    "features",
    "fzf",
    "items",
    "owner",
    "owners",
    "prime",
    "symbol",
    "tests",
    "workspace",
];

pub(crate) enum DevCommandLog {
    Disabled,
    Active(ActiveCommandLog),
}

pub(crate) struct ActiveCommandLog {
    argv: Vec<String>,
    binary: String,
    command: NormalizedCommand,
    context_source: String,
    cwd: PathBuf,
    event_id: String,
    hook_run_id: Option<String>,
    log_file: PathBuf,
    parent_event_id: Option<String>,
    project_root: PathBuf,
    project_root_hash: String,
    session_id: String,
    session_ordinal: u64,
    started_at_instant: Instant,
    started_at_system: SystemTime,
}

struct SessionContext {
    hook_run_id: Option<String>,
    parent_event_id: Option<String>,
    session_id: String,
    source: String,
}

#[derive(Clone)]
struct NormalizedCommand {
    namespace: String,
    method: String,
    pipes: Vec<String>,
    query: Option<String>,
    query_set_count: usize,
    render_mode: Option<String>,
    view: Option<String>,
}

impl DevCommandLog {
    pub(crate) fn start(argv: &[OsString]) -> Self {
        if !env_truthy("SEMANTIC_PROTOCOL_DEV_MODE") {
            return Self::Disabled;
        }

        let started_at_system = SystemTime::now();
        let started_at_instant = Instant::now();
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let project_root = infer_project_root(argv, &cwd).unwrap_or_else(|| cwd.clone());
        let project_root_hash = stable_hash_hex(&project_root.display().to_string());
        let log_root = match resolve_log_root(&project_root) {
            Some(path) => path,
            None => return Self::Disabled,
        };
        let session = resolve_session_context(&log_root, &project_root_hash);
        let session_ordinal = allocate_session_ordinal(&log_root, &session.session_id).unwrap_or(0);
        let event_id = make_event_id(started_at_system, session_ordinal);
        let log_file = log_root
            .join("rust")
            .join("rs-harness")
            .join("commands")
            .join(command_log_file_name(
                started_at_system,
                session_ordinal,
                &event_id,
            ));
        let argv = redact_argv(argv);
        let binary = argv
            .first()
            .map(|arg| binary_name(arg))
            .unwrap_or_else(|| "rs-harness".to_string());
        let command = normalize_command(&argv);

        Self::Active(ActiveCommandLog {
            argv,
            binary,
            command,
            context_source: session.source,
            cwd,
            event_id,
            hook_run_id: session.hook_run_id,
            log_file,
            parent_event_id: session.parent_event_id,
            project_root,
            project_root_hash,
            session_id: session.session_id,
            session_ordinal,
            started_at_instant,
            started_at_system,
        })
    }

    pub(crate) fn finish(self, exit_code: i32) {
        if let Self::Active(active) = self {
            let _ = write_event(&active, exit_code);
        }
    }
}

fn write_event(active: &ActiveCommandLog, exit_code: i32) -> io::Result<()> {
    let finished_at_system = SystemTime::now();
    let elapsed_ms = active
        .started_at_instant
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX));
    let parent = active.log_file.parent().ok_or_else(|| {
        io::Error::new(
            ErrorKind::InvalidInput,
            "dev command log file does not have a parent directory",
        )
    })?;
    fs::create_dir_all(parent)?;

    let mut line = String::new();
    let mut first = true;
    line.push('{');
    push_string_field(&mut line, &mut first, "schemaId", SCHEMA_ID);
    push_string_field(&mut line, &mut first, "schemaVersion", SCHEMA_VERSION);
    push_string_field(&mut line, &mut first, "protocolId", PROTOCOL_ID);
    push_string_field(&mut line, &mut first, "protocolVersion", PROTOCOL_VERSION);
    push_string_field(
        &mut line,
        &mut first,
        "timestampUtc",
        &format_utc_timestamp(finished_at_system),
    );
    push_string_field(
        &mut line,
        &mut first,
        "startedAtUtc",
        &format_utc_timestamp(active.started_at_system),
    );
    push_string_field(
        &mut line,
        &mut first,
        "finishedAtUtc",
        &format_utc_timestamp(finished_at_system),
    );
    push_string_field(&mut line, &mut first, "eventId", &active.event_id);
    push_string_field(&mut line, &mut first, "sessionId", &active.session_id);
    push_u64_field(
        &mut line,
        &mut first,
        "sessionOrdinal",
        active.session_ordinal,
    );
    if let Some(parent_event_id) = &active.parent_event_id {
        push_string_field(&mut line, &mut first, "parentEventId", parent_event_id);
    }
    push_string_field(&mut line, &mut first, "languageId", "rust");
    push_string_field(&mut line, &mut first, "providerId", "rs-harness");
    push_string_field(&mut line, &mut first, "binary", &active.binary);
    push_array_field(&mut line, &mut first, "argv", &active.argv);
    push_string_field(
        &mut line,
        &mut first,
        "cwd",
        &active.cwd.display().to_string(),
    );
    push_string_field(
        &mut line,
        &mut first,
        "projectRoot",
        &active.project_root.display().to_string(),
    );
    push_string_field(
        &mut line,
        &mut first,
        "projectRootHash",
        &active.project_root_hash,
    );
    if let Some(hook_run_id) = &active.hook_run_id {
        push_string_field(&mut line, &mut first, "hookRunId", hook_run_id);
    }
    push_command_field(&mut line, &mut first, &active.command);
    push_result_field(&mut line, &mut first, exit_code, elapsed_ms as u64);
    push_fields_field(&mut line, &mut first, active);
    line.push('}');
    line.push('\n');

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&active.log_file)?;
    file.write_all(line.as_bytes())
}

fn resolve_log_root(project_root: &Path) -> Option<PathBuf> {
    if let Some(value) = env_non_empty("SEMANTIC_PROTOCOL_TRACE_DIR") {
        return Some(path_from_env(&value, project_root));
    }
    if let Some(value) = env_non_empty("PRJ_CACHE_HOME") {
        return Some(path_from_env(&value, project_root).join("semantic_protocol"));
    }
    if let Some(value) = env_non_empty("XDG_CACHE_HOME") {
        return Some(
            PathBuf::from(value)
                .join("agent-semantic-protocols")
                .join(stable_hash_hex(&project_root.display().to_string()))
                .join("semantic_protocol"),
        );
    }
    env_non_empty("HOME").map(|home| {
        PathBuf::from(home)
            .join(".cache")
            .join("agent-semantic-protocols")
            .join(stable_hash_hex(&project_root.display().to_string()))
            .join("semantic_protocol")
    })
}

fn path_from_env(value: &str, project_root: &Path) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        project_root.join(path)
    }
}

fn allocate_session_ordinal(log_root: &Path, session_id: &str) -> Option<u64> {
    let dir = log_root.join("rust").join("rs-harness").join("sessions");
    fs::create_dir_all(&dir).ok()?;
    let key = sanitize_file_component(session_id);
    let counter_path = dir.join(format!("{key}.counter"));
    let lock_path = dir.join(format!("{key}.lock"));
    let _guard = acquire_lock(&lock_path)?;
    let current = fs::read_to_string(&counter_path)
        .ok()
        .and_then(|text| text.trim().parse::<u64>().ok())
        .unwrap_or(0);
    let next = current.saturating_add(1);
    fs::write(counter_path, next.to_string()).ok()?;
    Some(next)
}

struct LockGuard {
    path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn acquire_lock(lock_path: &Path) -> Option<LockGuard> {
    for _ in 0..50 {
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(lock_path)
        {
            Ok(mut file) => {
                let _ = writeln!(file, "{}", process::id());
                return Some(LockGuard {
                    path: lock_path.to_path_buf(),
                });
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(_) => return None,
        }
    }
    None
}

fn resolve_session_context(log_root: &Path, project_root_hash: &str) -> SessionContext {
    let env_session_id = env_first(&[
        "SEMANTIC_PROTOCOL_SESSION_ID",
        "CODEX_SESSION_ID",
        "CLAUDE_SESSION_ID",
        "TERM_SESSION_ID",
    ]);
    let env_hook_run_id = env_first(&[
        "SEMANTIC_PROTOCOL_HOOK_RUN_ID",
        "CODEX_HOOK_RUN_ID",
        "AGENT_HOOK_RUN_ID",
    ]);
    let env_parent_event_id = env_first(&["SEMANTIC_PROTOCOL_PARENT_EVENT_ID"]);
    if env_session_id.is_some() || env_hook_run_id.is_some() || env_parent_event_id.is_some() {
        let session_id = env_session_id.unwrap_or_else(|| {
            env_hook_run_id
                .as_ref()
                .map(|hook_run_id| format!("hook-{}", stable_hash_hex(hook_run_id)))
                .unwrap_or_else(|| format!("project-{project_root_hash}"))
        });
        let parent_event_id = env_parent_event_id.or_else(|| env_hook_run_id.clone());
        return SessionContext {
            hook_run_id: env_hook_run_id,
            parent_event_id,
            session_id,
            source: "env".to_string(),
        };
    }
    if let Some(context) = read_active_context(log_root, project_root_hash) {
        return context;
    }
    SessionContext {
        hook_run_id: None,
        parent_event_id: None,
        session_id: format!("project-{project_root_hash}"),
        source: "project-fallback".to_string(),
    }
}

fn read_active_context(log_root: &Path, project_root_hash: &str) -> Option<SessionContext> {
    let path = log_root
        .join("dev-context")
        .join(format!("{project_root_hash}.json"));
    let metadata = fs::metadata(&path).ok()?;
    let modified = metadata.modified().ok()?;
    if modified.elapsed().ok()? > Duration::from_secs(30 * 60) {
        return None;
    }
    let content = fs::read_to_string(path).ok()?;
    let session_id = extract_json_string(&content, "sessionId")
        .unwrap_or_else(|| format!("project-{project_root_hash}"));
    Some(SessionContext {
        hook_run_id: extract_json_string(&content, "hookRunId"),
        parent_event_id: extract_json_string(&content, "parentEventId"),
        session_id,
        source: "active-context".to_string(),
    })
}

fn extract_json_string(content: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let mut rest = content.split_once(&needle)?.1.trim_start();
    rest = rest.strip_prefix(':')?.trim_start();
    let mut chars = rest.chars();
    if chars.next()? != '"' {
        return None;
    }
    let mut value = String::new();
    let mut escaped = false;
    for ch in chars {
        if escaped {
            value.push(match ch {
                '"' => '"',
                '\\' => '\\',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(value);
        } else {
            value.push(ch);
        }
    }
    None
}

fn env_truthy(name: &str) -> bool {
    env::var(name)
        .map(|value| {
            matches!(
                value.as_str(),
                "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
            )
        })
        .unwrap_or(false)
}

fn env_non_empty(name: &str) -> Option<String> {
    env::var(name).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn env_first(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| env_non_empty(name))
}

fn infer_project_root(argv: &[OsString], cwd: &Path) -> Option<PathBuf> {
    for arg in argv.iter().skip(1).rev() {
        let text = arg.to_string_lossy();
        if text.starts_with('-') {
            continue;
        }
        let path = PathBuf::from(text.as_ref());
        let path = if path.is_absolute() {
            path
        } else {
            cwd.join(path)
        };
        if let Some(root) = project_root_from_path(&path) {
            return Some(root);
        }
    }
    project_root_from_path(cwd)
}

fn project_root_from_path(path: &Path) -> Option<PathBuf> {
    let mut cursor = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()?.to_path_buf()
    };
    loop {
        if PROJECT_ANCHORS
            .iter()
            .any(|anchor| cursor.join(anchor).exists())
        {
            return Some(fs::canonicalize(&cursor).unwrap_or(cursor));
        }
        if !cursor.pop() {
            return None;
        }
    }
}

fn redact_argv(argv: &[OsString]) -> Vec<String> {
    let mut redacted = Vec::with_capacity(argv.len());
    let mut iter = argv.iter().peekable();
    while let Some(arg) = iter.next() {
        let text = arg.to_string_lossy().to_string();
        if let Some((flag, _)) = text.split_once('=') {
            if is_secret_flag(flag) {
                redacted.push(format!("{flag}=[REDACTED]"));
                continue;
            }
        }
        if is_secret_flag(&text) {
            redacted.push(text);
            if iter.peek().is_some() {
                let _ = iter.next();
                redacted.push("[REDACTED]".to_string());
            }
            continue;
        }
        redacted.push(text);
    }
    redacted
}

fn is_secret_flag(flag: &str) -> bool {
    SECRET_FLAGS.iter().any(|candidate| candidate == &flag)
}

fn binary_name(arg: &str) -> String {
    Path::new(arg)
        .file_name()
        .and_then(|name| name.to_str())
        .map(normalize_token)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "rs-harness".to_string())
}

fn normalize_command(argv: &[String]) -> NormalizedCommand {
    let args = argv.iter().skip(1).cloned().collect::<Vec<_>>();
    let namespace_index = args.iter().position(|arg| !arg.starts_with('-'));
    let namespace = namespace_index
        .and_then(|index| args.get(index))
        .map(|arg| normalize_token(arg))
        .filter(|arg| !arg.is_empty())
        .unwrap_or_else(|| "cli".to_string());
    let render_mode = option_value(&args, "--view").map(|value| normalize_token(&value));
    let query_set_count = args
        .iter()
        .filter(|arg| arg.as_str() == "--query-set")
        .count()
        + args
            .iter()
            .filter(|arg| arg.starts_with("--query-set="))
            .count();
    let pipes = collect_pipes(&args);
    let view = if namespace == "search" {
        first_positional_after(&args, namespace_index.unwrap_or(0))
            .map(|value| normalize_token(&value))
    } else {
        None
    };
    let method = match namespace.as_str() {
        "search" => view
            .as_ref()
            .map(|view| format!("search/{view}"))
            .unwrap_or_else(|| "search".to_string()),
        "agent" => first_positional_after(&args, namespace_index.unwrap_or(0))
            .map(|subcommand| format!("agent/{}", normalize_token(&subcommand)))
            .unwrap_or_else(|| "agent".to_string()),
        other => other.to_string(),
    };
    let query = option_value(&args, "--query")
        .or_else(|| first_query_positional(&args, namespace_index.unwrap_or(0), view.as_deref()));

    NormalizedCommand {
        namespace,
        method,
        pipes,
        query,
        query_set_count,
        render_mode,
        view,
    }
}

fn first_positional_after(args: &[String], start: usize) -> Option<String> {
    let mut skip_next = false;
    for (index, arg) in args.iter().enumerate().skip(start + 1) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if option_takes_value(arg) {
            skip_next = !arg.contains('=');
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        if index > start {
            return Some(arg.clone());
        }
    }
    None
}

fn first_query_positional(
    args: &[String],
    namespace_index: usize,
    view: Option<&str>,
) -> Option<String> {
    let mut skip_next = false;
    let mut skipped_view = view.is_none();
    for arg in args.iter().skip(namespace_index + 1) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if option_takes_value(arg) {
            skip_next = !arg.contains('=');
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        let normalized = normalize_token(arg);
        if !skipped_view && Some(normalized.as_str()) == view {
            skipped_view = true;
            continue;
        }
        if SEARCH_PIPES.contains(&normalized.as_str()) {
            continue;
        }
        return Some(arg.clone());
    }
    None
}

fn option_value(args: &[String], name: &str) -> Option<String> {
    for (index, arg) in args.iter().enumerate() {
        if arg == name {
            return args.get(index + 1).cloned();
        }
        if let Some(value) = arg.strip_prefix(&format!("{name}=")) {
            return Some(value.to_string());
        }
    }
    None
}

fn option_takes_value(arg: &str) -> bool {
    let flag = arg.split_once('=').map(|(flag, _)| flag).unwrap_or(arg);
    VALUE_OPTIONS.contains(&flag)
}

fn collect_pipes(args: &[String]) -> Vec<String> {
    let mut pipes = BTreeSet::new();
    for arg in args {
        let token = normalize_token(arg);
        if SEARCH_PIPES.contains(&token.as_str()) {
            pipes.insert(token);
        }
    }
    pipes.into_iter().collect()
}

fn normalize_token(value: &str) -> String {
    let mut token = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            token.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            token.push(ch);
        }
    }
    if token.is_empty() {
        "unknown".to_string()
    } else {
        token
    }
}

fn make_event_id(started_at: SystemTime, session_ordinal: u64) -> String {
    format!(
        "rs-harness-{}-{}-{session_ordinal:06}",
        millis_since_epoch(started_at),
        process::id()
    )
}

fn command_log_file_name(started_at: SystemTime, session_ordinal: u64, event_id: &str) -> String {
    format!(
        "{}-{session_ordinal:06}-{}.jsonl",
        format_utc_file_timestamp(started_at),
        sanitize_file_component(event_id)
    )
}

fn sanitize_file_component(value: &str) -> String {
    let mut output = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            output.push(ch);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "unknown".to_string()
    } else {
        output
    }
}

fn stable_hash_hex(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn millis_since_epoch(time: SystemTime) -> u128 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis()
}

fn format_utc_timestamp(time: SystemTime) -> String {
    let (year, month, day, hour, minute, second) = utc_parts(time);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn format_utc_file_timestamp(time: SystemTime) -> String {
    let (year, month, day, hour, minute, second) = utc_parts(time);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}-{minute:02}-{second:02}Z")
}

fn utc_parts(time: SystemTime) -> (i32, u32, u32, u32, u32, u32) {
    let total_seconds = time
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64;
    let days = total_seconds.div_euclid(86_400);
    let seconds_of_day = total_seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = (seconds_of_day / 3_600) as u32;
    let minute = ((seconds_of_day % 3_600) / 60) as u32;
    let second = (seconds_of_day % 60) as u32;
    (year, month, day, hour, minute, second)
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

fn push_command_field(output: &mut String, first: &mut bool, command: &NormalizedCommand) {
    push_key(output, first, "command");
    let mut nested_first = true;
    output.push('{');
    push_string_field(output, &mut nested_first, "namespace", &command.namespace);
    push_string_field(output, &mut nested_first, "method", &command.method);
    if let Some(view) = &command.view {
        push_string_field(output, &mut nested_first, "view", view);
    }
    if let Some(render_mode) = &command.render_mode {
        push_string_field(output, &mut nested_first, "renderMode", render_mode);
    }
    if let Some(query) = &command.query {
        push_string_field(output, &mut nested_first, "query", query);
    }
    push_u64_field(
        output,
        &mut nested_first,
        "querySetCount",
        command.query_set_count as u64,
    );
    push_array_field(output, &mut nested_first, "pipes", &command.pipes);
    output.push('}');
}

fn push_result_field(output: &mut String, first: &mut bool, exit_code: i32, elapsed_ms: u64) {
    push_key(output, first, "result");
    let mut nested_first = true;
    output.push('{');
    push_i32_field(output, &mut nested_first, "exitCode", exit_code);
    push_u64_field(output, &mut nested_first, "elapsedMs", elapsed_ms);
    push_u64_field(output, &mut nested_first, "stdoutBytes", 0);
    push_u64_field(output, &mut nested_first, "stderrBytes", 0);
    push_string_field(
        output,
        &mut nested_first,
        "status",
        if exit_code == 0 { "success" } else { "failure" },
    );
    output.push('}');
}

fn push_fields_field(output: &mut String, first: &mut bool, active: &ActiveCommandLog) {
    push_key(output, first, "fields");
    let mut nested_first = true;
    output.push('{');
    push_bool_field(output, &mut nested_first, "outputBytesMeasured", false);
    push_string_field(
        output,
        &mut nested_first,
        "logFileNaming",
        "utc-second-session-ordinal-event",
    );
    push_string_field(output, &mut nested_first, "sequenceScope", "session");
    push_string_field(
        output,
        &mut nested_first,
        "contextSource",
        &active.context_source,
    );
    output.push('}');
}

fn push_string_field(output: &mut String, first: &mut bool, key: &str, value: &str) {
    push_key(output, first, key);
    push_json_string(output, value);
}

fn push_i32_field(output: &mut String, first: &mut bool, key: &str, value: i32) {
    push_key(output, first, key);
    output.push_str(&value.to_string());
}

fn push_u64_field(output: &mut String, first: &mut bool, key: &str, value: u64) {
    push_key(output, first, key);
    output.push_str(&value.to_string());
}

fn push_bool_field(output: &mut String, first: &mut bool, key: &str, value: bool) {
    push_key(output, first, key);
    output.push_str(if value { "true" } else { "false" });
}

fn push_array_field(output: &mut String, first: &mut bool, key: &str, values: &[String]) {
    push_key(output, first, key);
    output.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_json_string(output, value);
    }
    output.push(']');
}

fn push_key(output: &mut String, first: &mut bool, key: &str) {
    if *first {
        *first = false;
    } else {
        output.push(',');
    }
    push_json_string(output, key);
    output.push(':');
}

fn push_json_string(output: &mut String, value: &str) {
    output.push('"');
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch.is_control() => output.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => output.push(ch),
        }
    }
    output.push('"');
}
