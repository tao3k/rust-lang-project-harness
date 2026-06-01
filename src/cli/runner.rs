//! CLI runner, argument parsing, and semantic protocol dispatch.

use std::env;
#[cfg(feature = "search")]
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

use super::agent_registry::print_agent_registry;
#[cfg(feature = "search")]
use super::search_output::{SearchOutputControls, apply_search_output_controls};
#[cfg(feature = "search")]
use super::search_plan::{SearchPlanOptions, render_search_plan};
#[cfg(feature = "search")]
use super::search_trace::{SearchTraceOptions, render_search_trace};
#[cfg(feature = "search")]
use super::semantic_search_json::{SemanticSearchJsonOptions, render_search_json};
#[cfg(feature = "search")]
use crate::{
    RustHarnessConfig, RustSearchOptions, RustSearchViewRequest,
    render_rust_project_harness_search_ingest_with_config,
    render_rust_project_harness_search_view_with_config,
};
use crate::{
    render_rust_project_harness, render_rust_project_harness_agent_snapshot,
    render_rust_project_harness_json, run_rust_project_harness,
};

/// Run the CLI using process environment arguments.
#[must_use]
pub fn run_cli_from_env() -> ExitCode {
    match run(env::args_os().skip(1)) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn run(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<ExitCode, String> {
    let args = args.into_iter().collect::<Vec<_>>();
    if is_command(&args, "search") {
        return run_search(args.into_iter().skip(1));
    }
    if is_command(&args, "check") {
        return run_check(args.into_iter().skip(1));
    }
    if is_command(&args, "agent") {
        return run_agent(args.into_iter().skip(1));
    }

    let options = CliOptions::parse(args)?;
    if options.help {
        print_help();
        return Ok(ExitCode::SUCCESS);
    }
    let project_root = options.project_root()?;
    let report = run_rust_project_harness(&project_root)?;
    if options.agent_snapshot {
        print!(
            "{}",
            render_rust_project_harness_agent_snapshot(&project_root)?
        );
    } else if options.json {
        println!(
            "{}",
            render_rust_project_harness_json(&report)
                .map_err(|error| format!("failed to render JSON report: {error}"))?
        );
    } else {
        print!("{}", render_rust_project_harness(&report));
    }
    if report.is_clean() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn run_check(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<ExitCode, String> {
    let options = CheckOptions::parse(args)?;
    if options.help {
        print_check_help();
        return Ok(ExitCode::SUCCESS);
    }
    let _mode = options
        .mode
        .as_deref()
        .ok_or_else(|| "expected check mode: --changed or --full".to_string())?;
    let project_root = options.project_root()?;
    let report = run_rust_project_harness(&project_root)?;
    if options.json {
        println!(
            "{}",
            render_rust_project_harness_json(&report)
                .map_err(|error| format!("failed to render JSON report: {error}"))?
        );
    } else {
        print!("{}", render_rust_project_harness(&report));
    }
    if report.is_clean() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn run_agent(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<ExitCode, String> {
    let options = AgentOptions::parse(args)?;
    if options.help {
        print_agent_help();
        return Ok(ExitCode::SUCCESS);
    }
    let project_root = options.project_root()?;
    match options.command.as_str() {
        "install" => {
            return Err(moved_agent_action("install"));
        }
        "doctor" => {
            if options.json {
                print_agent_registry(&project_root)?;
            } else {
                print_agent_doctor(&project_root, options.client.as_deref());
            }
        }
        "hook" => {
            let _ = options.hook_event.as_deref();
            return Err(moved_agent_action("hook"));
        }
        "guard" => {
            let _ = options.guard_args.len();
            return Err(moved_agent_action("guard"));
        }
        "guide" => {
            let _ = options.client.as_deref();
            print_agent_guide(&project_root);
        }
        other => return Err(format!("unknown agent command: {other}")),
    }
    Ok(ExitCode::SUCCESS)
}

fn run_search(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<ExitCode, String> {
    let options = SearchOptions::parse(args)?;
    if options.help {
        print_search_help();
        return Ok(ExitCode::SUCCESS);
    }
    run_search_view(&options)
}

#[cfg(feature = "search")]
fn run_search_view(options: &SearchOptions) -> Result<ExitCode, String> {
    let project_root = options.project_root()?;
    let render_options = options.render_options();
    let config = RustHarnessConfig::default();
    let raw_rendered = if options.view == "ingest" {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| format!("failed to read search ingest stdin: {error}"))?;
        render_rust_project_harness_search_ingest_with_config(
            &project_root,
            &config,
            &input,
            &render_options,
        )?
    } else {
        let request = RustSearchViewRequest {
            project_root: &project_root,
            config: &config,
            view: &options.view,
            query: options.query.as_deref(),
            options: &render_options,
        };
        render_rust_project_harness_search_view_with_config(&request)?
    };
    let rendered = apply_search_output_controls(
        SearchOutputControls {
            depth: options.depth,
            output_view: options.output_view.as_deref(),
            seeds: options.seeds,
        },
        &raw_rendered,
    );
    if options.json {
        let json_options = options.semantic_json_options();
        println!(
            "{}",
            render_search_json(&project_root, &json_options, &rendered)?
        );
    } else {
        if options.trace {
            let trace_options = options.search_trace_options();
            print!("{}", render_search_trace(&trace_options, &raw_rendered));
        }
        if options.explain {
            print!("{}", render_search_plan(options.search_plan_options()));
        }
        print!("{rendered}");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(not(feature = "search"))]
fn run_search_view(_options: &SearchOptions) -> Result<ExitCode, String> {
    Err("search command requires the `search` feature".to_string())
}

#[derive(Debug, Default)]
struct CliOptions {
    json: bool,
    agent_snapshot: bool,
    help: bool,
    paths: Vec<PathBuf>,
}

#[derive(Debug, Default)]
struct CheckOptions {
    json: bool,
    help: bool,
    mode: Option<String>,
    paths: Vec<PathBuf>,
}

#[derive(Debug)]
struct AgentOptions {
    command: String,
    hook_event: Option<String>,
    client: Option<String>,
    guard_args: Vec<std::ffi::OsString>,
    json: bool,
    help: bool,
    paths: Vec<PathBuf>,
}

impl Default for AgentOptions {
    fn default() -> Self {
        Self {
            command: "doctor".to_string(),
            hook_event: None,
            client: None,
            guard_args: Vec::new(),
            json: false,
            help: false,
            paths: Vec::new(),
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct SearchOptions {
    pub(super) view: String,
    pub(super) query: Option<String>,
    pub(super) json: bool,
    pub(super) help: bool,
    pub(super) trace: bool,
    pub(super) explain: bool,
    pub(super) output_view: Option<String>,
    pub(super) depth: Option<usize>,
    pub(super) dir: Option<String>,
    pub(super) edges: Vec<String>,
    pub(super) per_owner: Option<usize>,
    pub(super) seeds: Option<usize>,
    pub(super) owners: Option<usize>,
    pub(super) hits: Option<usize>,
    pub(super) package: Option<String>,
    pub(super) owner: Option<String>,
    pub(super) dependency: Option<String>,
    pub(super) scope: Option<String>,
    pub(super) lines: bool,
    pub(super) pipes: Vec<String>,
    pub(super) query_set: Vec<String>,
    paths: Vec<PathBuf>,
}

impl SearchOptions {
    fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut positional_only = false;
        let mut pending_option: Option<String> = None;
        let mut positionals = Vec::<std::ffi::OsString>::new();
        for arg in args {
            if let Some(option) = pending_option.take() {
                let Some(value) = arg.to_str() else {
                    return Err(format!("expected UTF-8 value after {option}"));
                };
                match option.as_str() {
                    "--package" => options.package = Some(value.to_string()),
                    "--owner" => options.owner = Some(value.to_string()),
                    "--dependency" => options.dependency = Some(value.to_string()),
                    "--scope" => options.scope = Some(value.to_string()),
                    "--view" => options.output_view = Some(value.to_string()),
                    "--depth" => options.depth = Some(parse_usize_option(&option, value)?),
                    "--dir" => options.dir = Some(value.to_string()),
                    "--edge" => options.edges.extend(split_csv_values(value)),
                    "--per-owner" => {
                        options.per_owner = Some(parse_usize_option(&option, value)?);
                    }
                    "--seeds" => options.seeds = Some(parse_usize_option(&option, value)?),
                    "--owners" => options.owners = Some(parse_usize_option(&option, value)?),
                    "--hits" => options.hits = Some(parse_usize_option(&option, value)?),
                    "--query-set" => options.query_set.extend(split_csv_values(value)),
                    _ => unreachable!("known pending search option"),
                }
                continue;
            }
            if positional_only {
                positionals.push(arg);
                continue;
            }
            let Some(value) = arg.to_str() else {
                positionals.push(arg);
                continue;
            };
            match value {
                "--" => positional_only = true,
                "--json" => options.json = true,
                "--help" | "-h" => options.help = true,
                "--lines" => options.lines = true,
                "--trace" => options.trace = true,
                "--explain" => options.explain = true,
                "--item-slice" => options.pipes.push("items".to_string()),
                "--package" | "--owner" | "--dependency" | "--scope" | "--view" | "--depth"
                | "--dir" | "--edge" | "--per-owner" | "--seeds" | "--owners" | "--hits"
                | "--query-set" => pending_option = Some(value.to_string()),
                value if value.starts_with('-') => {
                    return Err(format!("unknown search option: {value}"));
                }
                _ => positionals.push(arg),
            }
        }
        if let Some(option) = pending_option {
            return Err(format!("expected value after {option}"));
        }
        if options.help {
            return Ok(options);
        }
        options.apply_positionals(positionals)?;
        if options.view.is_empty() {
            return Err("expected search view: prime".to_string());
        }
        if options.paths.len() > 1 {
            return Err("expected at most one PROJECT_ROOT argument".to_string());
        }
        validate_search_options(&options)?;
        Ok(options)
    }

    fn apply_positionals(&mut self, positionals: Vec<std::ffi::OsString>) -> Result<(), String> {
        let mut values = positionals
            .into_iter()
            .map(|value| {
                value
                    .into_string()
                    .map_err(|_| "expected UTF-8 search arguments".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        if values.is_empty() {
            return Ok(());
        }
        self.view = values.remove(0);
        if search_view_requires_query(&self.view) {
            if !self.query_set.is_empty() {
                self.query = Some(self.query_set.join(","));
            } else {
                if values.is_empty() {
                    return Err(format!("search {} requires a query", self.command_label()));
                }
                self.query = Some(values.remove(0));
            }
        } else if search_view_accepts_optional_query(&self.view)
            && values
                .first()
                .is_some_and(|value| !is_search_pipe(value) && !PathBuf::from(value).exists())
        {
            self.query = Some(values.remove(0));
        }
        for value in values {
            if is_search_pipe(&value) {
                self.pipes.push(value);
            } else {
                self.paths.push(PathBuf::from(value));
            }
        }
        Ok(())
    }

    pub(super) fn command_label(&self) -> String {
        self.view.clone()
    }

    #[cfg(feature = "search")]
    fn search_plan_options(&self) -> SearchPlanOptions<'_> {
        SearchPlanOptions {
            view: &self.view,
            query: self.query.as_deref(),
            output_view: self.output_view.as_deref(),
            depth: self.depth,
            dir: self.dir.as_deref(),
            edges: &self.edges,
            pipes: &self.pipes,
        }
    }

    #[cfg(feature = "search")]
    fn semantic_json_options(&self) -> SemanticSearchJsonOptions {
        SemanticSearchJsonOptions {
            view: self.view.clone(),
            query: self.query.clone(),
            command_label: self.command_label(),
            trace: self.trace,
            explain: self.explain,
            output_view: self.output_view.clone(),
            depth: self.depth,
            dir: self.dir.clone(),
            edges: self.edges.clone(),
            per_owner: self.per_owner,
            seeds: self.seeds,
            owners: self.owners,
            hits: self.hits,
            package: self.package.clone(),
            owner: self.owner.clone(),
            dependency: self.dependency.clone(),
            scope: self.scope.clone(),
            lines: self.lines,
            pipes: self.pipes.clone(),
            query_set: self.query_set(),
        }
    }

    #[cfg(feature = "search")]
    fn search_trace_options(&self) -> SearchTraceOptions {
        SearchTraceOptions {
            source: self.command_label(),
            query: self.query.clone(),
            pipes: self.pipes.clone(),
            output_view: self.output_view.clone(),
        }
    }

    #[cfg(feature = "search")]
    fn render_options(&self) -> RustSearchOptions {
        RustSearchOptions {
            package: self.package.clone(),
            owner: self.owner.clone(),
            dependency: self.dependency.clone(),
            scope: self.scope.clone(),
            pipes: self.pipes.clone(),
            lines: self.lines,
            output_view: self.output_view.clone(),
            seeds: self.seeds,
            query_set: self.query_set(),
        }
    }

    #[cfg(feature = "search")]
    fn project_root(&self) -> Result<PathBuf, String> {
        match self.paths.as_slice() {
            [path] => Ok(path.clone()),
            [] => {
                env::current_dir().map_err(|error| format!("failed to read current dir: {error}"))
            }
            _ => unreachable!("parse enforces at most one path"),
        }
    }

    fn query_set(&self) -> Vec<String> {
        if !search_view_supports_query_set(&self.view) {
            return Vec::new();
        }
        if !self.query_set.is_empty() {
            return self.query_set.clone();
        }
        let Some(query) = self.query.as_deref() else {
            return Vec::new();
        };
        let terms = split_csv_values(query);
        if terms.len() > 1 { terms } else { Vec::new() }
    }
}

impl CheckOptions {
    fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut positional_only = false;
        for arg in args {
            if positional_only {
                options.paths.push(PathBuf::from(arg));
                continue;
            }
            let Some(value) = arg.to_str() else {
                options.paths.push(PathBuf::from(arg));
                continue;
            };
            match value {
                "--" => positional_only = true,
                "--json" => options.json = true,
                "--help" | "-h" => options.help = true,
                "--changed" => options.set_mode("changed")?,
                "--full" => options.set_mode("full")?,
                value if value.starts_with('-') => {
                    return Err(format!("unknown check option: {value}"));
                }
                _ => options.paths.push(PathBuf::from(arg)),
            }
        }
        if options.paths.len() > 1 {
            return Err("expected at most one PROJECT_ROOT argument".to_string());
        }
        Ok(options)
    }

    fn project_root(&self) -> Result<PathBuf, String> {
        match self.paths.as_slice() {
            [path] => Ok(path.clone()),
            [] => {
                env::current_dir().map_err(|error| format!("failed to read current dir: {error}"))
            }
            _ => unreachable!("parse enforces at most one path"),
        }
    }

    fn set_mode(&mut self, mode: &str) -> Result<(), String> {
        if self.mode.is_some() {
            return Err("expected only one check mode: --changed or --full".to_string());
        }
        self.mode = Some(mode.to_string());
        Ok(())
    }
}

impl AgentOptions {
    fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut positional_only = false;
        let mut pending_option: Option<String> = None;
        for arg in args {
            if let Some(option) = pending_option.take() {
                let Some(value) = arg.to_str() else {
                    return Err(format!("expected UTF-8 value for {option}"));
                };
                match option.as_str() {
                    "--client" => options.set_client(value)?,
                    "--scope" | "--profile" => {
                        return Err(
                            "rs-harness no longer writes Codex hook configs; use semantic-agent-hook install --client codex".to_string(),
                        );
                    }
                    _ => unreachable!("unknown pending option"),
                }
                continue;
            }
            if positional_only {
                if options.command == "guard" {
                    options.guard_args.push(arg);
                } else {
                    options.paths.push(PathBuf::from(arg));
                }
                continue;
            }
            let Some(value) = arg.to_str() else {
                options.paths.push(PathBuf::from(arg));
                continue;
            };
            match value {
                "--" => positional_only = true,
                "--json" => options.json = true,
                "--client" => pending_option = Some(value.to_string()),
                "--scope" | "--profile" => pending_option = Some(value.to_string()),
                "--codex" => options.set_client("codex")?,
                "--help" | "-h" => options.help = true,
                value if value.starts_with('-') => {
                    return Err(format!("unknown agent option: {value}"));
                }
                "install" | "doctor" | "hook" | "guide" | "guard"
                    if options.command == "doctor" =>
                {
                    options.command = value.to_string();
                }
                value if options.command == "hook" && options.hook_event.is_none() => {
                    options.hook_event = Some(value.to_string());
                }
                _ => options.paths.push(PathBuf::from(arg)),
            }
        }
        if let Some(option) = pending_option {
            return Err(format!("expected value after {option}"));
        }
        if options.paths.len() > 1 {
            return Err("expected at most one PROJECT_ROOT argument".to_string());
        }
        Ok(options)
    }

    fn project_root(&self) -> Result<PathBuf, String> {
        match self.paths.as_slice() {
            [path] => Ok(path.clone()),
            [] => {
                env::current_dir().map_err(|error| format!("failed to read current dir: {error}"))
            }
            _ => unreachable!("parse enforces at most one path"),
        }
    }

    fn set_client(&mut self, client: &str) -> Result<(), String> {
        if self
            .client
            .as_deref()
            .is_some_and(|existing| existing != client)
        {
            return Err("expected only one agent client".to_string());
        }
        self.client = Some(client.to_string());
        Ok(())
    }
}

impl CliOptions {
    fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut positional_only = false;
        for arg in args {
            if positional_only {
                options.paths.push(PathBuf::from(arg));
                continue;
            }
            let Some(value) = arg.to_str() else {
                options.paths.push(PathBuf::from(arg));
                continue;
            };
            match value {
                "--" => positional_only = true,
                "--json" => options.json = true,
                "--agent-snapshot" => options.agent_snapshot = true,
                "--help" | "-h" => options.help = true,
                value if value.starts_with('-') => {
                    return Err(format!("unknown option: {value}"));
                }
                _ => options.paths.push(PathBuf::from(arg)),
            }
        }
        if options.paths.len() > 1 {
            return Err("expected at most one PROJECT_ROOT argument".to_string());
        }
        if options.json && options.agent_snapshot {
            return Err("expected only one output mode: --json or --agent-snapshot".to_string());
        }
        Ok(options)
    }

    fn project_root(&self) -> Result<PathBuf, String> {
        match self.paths.as_slice() {
            [path] => Ok(path.clone()),
            [] => {
                env::current_dir().map_err(|error| format!("failed to read current dir: {error}"))
            }
            _ => unreachable!("parse enforces at most one path"),
        }
    }
}

fn print_help() {
    println!(
        "rs-harness [--json | --agent-snapshot] [PROJECT_ROOT]\n\
         rs-harness search <view> [ARGS] [PIPE...] [--json] [--package PACKAGE] [PROJECT_ROOT]\n\
         rs-harness check <--changed|--full> [--json] [PROJECT_ROOT]\n\
         rs-harness agent doctor [--json] [PROJECT_ROOT]\n\
         rs-harness agent guide [PROJECT_ROOT]\n\n\
         Runs the default package-level Rust harness.\n\n\
         Compact text is the default agent-facing repair surface.\n\
         Use --json to emit the structured RustHarnessReport audit shape.\n\
         Use --agent-snapshot to emit a low-noise reasoning-tree summary.\n\
         Use search for RFC line-protocol exploration views."
    );
}

fn print_search_help() {
    println!(
        "rs-harness search prime [--package PACKAGE] [PROJECT_ROOT]\n\
         rs-harness search owner <path-or-owner> [items] [--scope SCOPE] [PROJECT_ROOT]\n\
         rs-harness search workspace [--package PACKAGE] [PROJECT_ROOT]\n\
         rs-harness search targets [--package PACKAGE] [PROJECT_ROOT]\n\
         rs-harness search deps [dep[/subpath][@version][::api]] [public-api] [PROJECT_ROOT]\n\
         rs-harness search features [feature] [cfg owners tests] [PROJECT_ROOT]\n\
         rs-harness search dependency <crate-or-import-or-package> [items public-api docs tests] [PROJECT_ROOT]\n\
         rs-harness search <symbol|callsite|import|text|cfg|pattern|docs|docs-use|api> <query> [PROJECT_ROOT]\n\
         rs-harness search <owner|dependency|text|tests> --query-set TERM [--query-set TERM...] [PROJECT_ROOT]\n\
         rs-harness search public-external-types [--dependency DEP] [PROJECT_ROOT]\n\
         rg -n '<query>' src tests | rs-harness search ingest [items tests] [PROJECT_ROOT]\n\n\
         Emits compact RFC line protocol for deterministic agent exploration.\n\
         Compact text is the default; --json wraps the same packet for tools.\n\
         RFC controls accepted here: --trace, --explain, --view graph|hits|both|seeds,\n\
         --depth N, --dir out|in|both, --edge LIST, --item-slice, --dependency DEP,\n\
         --seeds N, --query-set TERM, --lines."
    );
}

fn print_check_help() {
    println!(
        "rs-harness check --changed [--json] [PROJECT_ROOT]\n\
         rs-harness check --full [--json] [PROJECT_ROOT]\n\n\
         Runs the policy surface and renders compact findings by default."
    );
}

fn print_agent_help() {
    println!(
        "rs-harness agent doctor [--json] [PROJECT_ROOT]\n\
         rs-harness agent guide [PROJECT_ROOT]\n\n\
         Hook install/runtime is owned by semantic-agent-hook in the root toolchain.\n\
         agent guide prints the command-line search flow guide used by hooks.\n\
         Use --json to emit the semantic-language registry contract."
    );
}

fn moved_agent_action(action: &str) -> String {
    if action == "guard" {
        return "rs-harness agent guard moved to semantic-agent-hook; use semantic-agent-hook hook --client codex pre-tool --emit decision".to_string();
    }
    format!(
        "rs-harness agent {action} moved to semantic-agent-hook; use semantic-agent-hook {action} --client codex"
    )
}

fn print_agent_doctor(project_root: &std::path::Path, _client: Option<&str>) {
    println!(
        "[agent-doctor] status=ok provider=rs-harness runtime=semantic-agent-hook project={}",
        project_root.display()
    );
}

fn print_agent_guide(_project_root: &std::path::Path) {
    println!(
        "[agent-guide] runtime=semantic-agent-hook language=rust provider=rs-harness\n\
         |flow prime->owner|deps|symbol|tests pipe=text:tests ingest=stdin\n\
         |cmd prime=rs-harness search prime --view seeds .\n\
         |cmd owner=rs-harness search owner <path> items --view seeds .\n\
         |cmd ingest=rg -n '<query>' src tests | rs-harness search ingest items tests --view seeds .\n\
         |rule hook install/runtime is owned by semantic-agent-hook"
    );
}

fn is_command(args: &[std::ffi::OsString], command: &str) -> bool {
    args.first()
        .and_then(|arg| arg.to_str())
        .is_some_and(|arg| arg == command)
}

fn search_view_requires_query(view: &str) -> bool {
    matches!(
        view,
        "owner"
            | "dependency"
            | "tests"
            | "symbol"
            | "callsite"
            | "import"
            | "text"
            | "cfg"
            | "pattern"
            | "docs"
            | "docs-use"
            | "api"
    )
}

fn search_view_accepts_optional_query(view: &str) -> bool {
    matches!(view, "deps" | "features")
}

fn search_view_supports_query_set(view: &str) -> bool {
    matches!(view, "owner" | "dependency" | "text" | "tests")
}

fn is_search_pipe(value: &str) -> bool {
    matches!(
        value,
        "owners"
            | "usage"
            | "items"
            | "tests"
            | "examples"
            | "benches"
            | "docs"
            | "docs-use"
            | "api"
            | "public-external-types"
            | "public-api"
            | "cfg"
            | "features"
            | "dependents"
    )
}

fn parse_usize_option(option: &str, value: &str) -> Result<usize, String> {
    value
        .parse()
        .map_err(|_| format!("expected integer value after {option}"))
}

fn split_csv_values(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn validate_search_options(options: &SearchOptions) -> Result<(), String> {
    if let Some(view) = options.output_view.as_deref()
        && !matches!(view, "graph" | "hits" | "both" | "seeds")
    {
        return Err(format!("unknown search --view mode: {view}"));
    }
    if let Some(dir) = options.dir.as_deref()
        && !matches!(dir, "out" | "in" | "both")
    {
        return Err(format!("unknown search --dir mode: {dir}"));
    }
    if !options.query_set.is_empty() && !search_view_supports_query_set(&options.view) {
        return Err(format!(
            "search {} does not support --query-set",
            options.command_label()
        ));
    }
    Ok(())
}
