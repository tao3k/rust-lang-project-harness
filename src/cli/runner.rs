//! CLI runner, argument parsing, and semantic protocol dispatch.

use std::env;
#[cfg(feature = "search")]
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

use super::agent_registry::print_agent_registry;
use super::flow_lite_query::run_flow_lite_query_catalog;
use super::query::{
    QueryCommand, QueryGuideKind, parse_query, print_query_guide, print_query_help,
    print_tree_sitter_query_guide, query_guide_kind, render_query_local_item_code,
    render_query_local_window,
};
use super::query_options::QuerySearchOptions;
use super::query_source::QuerySourceVersion;
use super::query_window::render_query_local_item_frontier;
use super::runner_support::{
    discover_rust_project_root, is_command, is_explicit_rust_project_root, is_known_search_view,
    is_search_pipe, moved_agent_action, parse_usize_option, print_agent_doctor, print_agent_help,
    print_check_help, print_guide, print_help, print_search_help, rust_project_root_for_path,
    search_view_accepts_optional_query, search_view_requires_query, search_view_supports_query_set,
    split_csv_values,
};
#[cfg(feature = "search")]
use super::search_output::{
    SearchOutputControls, apply_search_output_controls, render_search_graph_packet,
};
#[cfg(feature = "search")]
use super::search_plan::{SearchPlanOptions, render_search_plan};
#[cfg(feature = "search")]
use super::search_trace::{SearchTraceOptions, render_search_trace};
#[cfg(feature = "search")]
use super::semantic_query_json::{SemanticQueryJsonOptions, render_query_json};
#[cfg(feature = "search")]
use super::semantic_search_json::{
    SemanticSearchJsonOptions, build_search_packet, render_search_json,
};
use super::tree_sitter_query::run_tree_sitter_query_catalog;
#[cfg(feature = "search")]
use crate::{
    RustHarnessConfig, RustSearchOptions, RustSearchViewRequest,
    render_rust_project_harness_search_ingest_with_config,
    render_rust_project_harness_search_semantic_facts_json,
    render_rust_project_harness_search_view_with_config,
};
use crate::{
    render_rust_project_harness, render_rust_project_harness_agent_snapshot,
    render_rust_project_harness_failure_frontier, render_rust_project_harness_json,
    run_rust_project_harness, rust_harness_config_for_project,
};

/// Run the Rust harness CLI from process arguments and return its exit code.
pub fn run_cli_from_env() -> ExitCode {
    let argv = env::args_os().collect::<Vec<_>>();
    let log = super::dev_command_log::DevCommandLog::start(&argv);
    let result = match run(argv.iter().skip(1).cloned()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    };
    log.finish(if result == ExitCode::SUCCESS { 0 } else { 2 });
    result
}

fn run(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<ExitCode, String> {
    let args = args.into_iter().collect::<Vec<_>>();
    if is_command(&args, "search") {
        return run_search(args.into_iter().skip(1));
    }
    if is_command(&args, "query") {
        return run_query(args.into_iter().skip(1));
    }
    if is_command(&args, "check") {
        return run_check(args.into_iter().skip(1));
    }
    if is_command(&args, "behavior") {
        return super::behavior_snapshot::run_behavior(args.into_iter().skip(1));
    }
    if is_command(&args, "determinism") {
        return super::determinism_readiness::run_determinism(args.into_iter().skip(1));
    }
    if is_command(&args, "receipt") {
        return super::execution_receipt::run_receipt(args.into_iter().skip(1));
    }
    if is_command(&args, "ast-patch") {
        return super::ast_patch::run_ast_patch(args.into_iter().skip(1));
    }
    if is_command(&args, "proof") {
        return super::formal_proof_pilot::run_proof(args.into_iter().skip(1));
    }
    if is_command(&args, "review") {
        return super::review_packet::run_review(args.into_iter().skip(1));
    }
    if is_command(&args, "evidence") {
        return super::evidence_graph::run_evidence(args.into_iter().skip(1));
    }
    if is_command(&args, "guide") {
        let options = CliOptions::parse(args.into_iter().skip(1))?;
        if options.help {
            print_help();
            return Ok(ExitCode::SUCCESS);
        }
        print_guide(&options.project_root()?);
        return Ok(ExitCode::SUCCESS);
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
    let mode = options
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
    } else if mode == "changed" {
        let frontier = render_rust_project_harness_failure_frontier(&report, &project_root, 4);
        if frontier.is_empty() {
            print!("{}", render_rust_project_harness(&report));
        } else {
            print!("{frontier}");
        }
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

fn run_query(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<ExitCode, String> {
    let args = args.into_iter().collect::<Vec<_>>();
    if let Some(kind) = query_guide_kind(&args) {
        match kind {
            QueryGuideKind::Query => print_query_guide(),
            QueryGuideKind::TreeSitter => print_tree_sitter_query_guide(),
        }
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(exit_code) = run_flow_lite_query_catalog(&args)? {
        return Ok(exit_code);
    }
    if let Some(exit_code) = run_tree_sitter_query_catalog(&args)? {
        return Ok(exit_code);
    }

    match parse_query(args)? {
        QueryCommand::Help => {
            print_query_help();
            Ok(ExitCode::SUCCESS)
        }
        QueryCommand::Search(options) => {
            let search_options = SearchOptions::from_query(*options);
            run_query_view(&search_options)
        }
    }
}

#[cfg(feature = "search")]
fn run_search_view(options: &SearchOptions) -> Result<ExitCode, String> {
    let project_root = options.project_root()?;
    let render_options = options.render_options();
    let config = rust_harness_config_for_project(&project_root);
    if options.view == "semantic-facts" {
        if !options.json {
            return Err("search semantic-facts requires --json".to_string());
        }
        let query = options
            .query
            .as_deref()
            .ok_or_else(|| "search semantic-facts requires a query".to_string())?;
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| format!("failed to read search semantic-facts stdin: {error}"))?;
        print!(
            "{}",
            render_rust_project_harness_search_semantic_facts_json(&project_root, query, &input)?
        );
        return Ok(ExitCode::SUCCESS);
    }
    let raw_rendered = if let Some(rendered) =
        render_search_owner_item_frontier_fast_path(&project_root, options)?
    {
        rendered
    } else if options.view == "ingest" {
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
    let json_options = options.semantic_json_options();
    let rendered = apply_search_output_controls(
        SearchOutputControls {
            depth: options.depth,
            output_view: options.output_view.as_deref(),
            seeds: options.seeds,
        },
        &raw_rendered,
    );
    let rendered = if options.output_view.as_deref() == Some("seeds") {
        let packet = build_search_packet(&project_root, &json_options, &raw_rendered);
        render_search_graph_packet(&packet, options.seeds)?
    } else {
        rendered
    };
    if options.json {
        println!(
            "{}",
            render_search_json(&project_root, &json_options, &raw_rendered)?
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

#[cfg(feature = "search")]
fn render_search_owner_item_frontier_fast_path(
    project_root: &std::path::Path,
    options: &SearchOptions,
) -> Result<Option<String>, String> {
    if options.view != "owner"
        || options.output_view.as_deref() != Some("seeds")
        || options.json
        || options.trace
        || options.explain
        || !options.pipes.iter().any(|pipe| pipe == "items")
    {
        return Ok(None);
    }
    let (Some(selector), Some(item_query)) =
        (options.query.as_deref(), options.item_query.as_deref())
    else {
        return Ok(None);
    };
    render_query_local_item_frontier(
        project_root,
        selector,
        item_query,
        options.source_version,
        false,
    )
}

#[cfg(not(feature = "search"))]
fn run_search_view(_options: &SearchOptions) -> Result<ExitCode, String> {
    Err("search command requires the `search` feature".to_string())
}

#[cfg(feature = "search")]
fn run_query_view(options: &SearchOptions) -> Result<ExitCode, String> {
    if options.item_code
        && let (Some(selector), Some(item_query)) =
            (options.query.as_deref(), options.item_query.as_deref())
    {
        let project_root = options.project_root()?;
        if let Some(rendered) = render_query_local_item_code(
            &project_root,
            selector,
            item_query,
            options.source_version,
        )? {
            print!("{rendered}");
            return Ok(ExitCode::SUCCESS);
        }
    }
    if options.item_names_only
        && let (Some(selector), Some(item_query)) =
            (options.query.as_deref(), options.item_query.as_deref())
    {
        let project_root = options.project_root()?;
        if let Some(rendered) = render_query_local_item_frontier(
            &project_root,
            selector,
            item_query,
            options.source_version,
            true,
        )? {
            if options.json {
                let json_options = options.semantic_query_json_options()?;
                println!(
                    "{}",
                    render_query_json(&project_root, &json_options, &rendered)?
                );
            } else {
                print!("{rendered}");
            }
            return Ok(ExitCode::SUCCESS);
        }
    }

    let local_window_selector = options
        .read_selector
        .as_deref()
        .filter(|_| options.item_code || options.item_query.is_none())
        .or({
            if options.item_code {
                options.query.as_deref()
            } else {
                None
            }
        });
    if let Some(selector) = local_window_selector {
        let project_root = options.project_root()?;
        if let Some(rendered) = render_query_local_window(
            &project_root,
            selector,
            options.item_code,
            options.source_version,
        )? {
            if options.json && options.output_view.as_deref() == Some("read-packet") {
                let read_options = super::semantic_read_json::SemanticReadJsonOptions {
                    selector: options
                        .read_selector
                        .clone()
                        .or_else(|| options.owner.clone())
                        .unwrap_or_else(|| selector.to_string()),
                    query: options.item_query.clone(),
                    source_version: options.source_version,
                };
                println!(
                    "{}",
                    super::semantic_read_json::render_read_json(
                        &project_root,
                        &read_options,
                        &rendered
                    )?
                );
            } else {
                print!("{rendered}");
            }
            return Ok(ExitCode::SUCCESS);
        }
    }

    let project_root = options.project_root()?;
    let mut render_options = options.render_options();
    if options.json
        && options.output_view.as_deref() != Some("read-packet")
        && options.view == "owner"
        && options.item_query.is_some()
    {
        render_options.item_projection_metadata = true;
    }
    let config = RustHarnessConfig::default();
    let request = RustSearchViewRequest {
        project_root: &project_root,
        config: &config,
        view: &options.view,
        query: options.query.as_deref(),
        options: &render_options,
    };
    let raw_rendered = render_rust_project_harness_search_view_with_config(&request)?;
    let rendered = apply_search_output_controls(
        SearchOutputControls {
            depth: options.depth,
            output_view: options.output_view.as_deref(),
            seeds: options.seeds,
        },
        &raw_rendered,
    );
    let rendered = if options.output_view.as_deref() == Some("seeds") {
        let json_options = options.semantic_json_options();
        let packet = build_search_packet(&project_root, &json_options, &raw_rendered);
        render_search_graph_packet(&packet, options.seeds)?
    } else {
        rendered
    };
    if options.json && options.output_view.as_deref() == Some("read-packet") {
        let read_options = super::semantic_read_json::SemanticReadJsonOptions {
            selector: options
                .read_selector
                .clone()
                .or_else(|| options.owner.clone())
                .or_else(|| options.query.clone())
                .unwrap_or_else(|| ".".to_string()),
            query: options.item_query.clone(),
            source_version: options.source_version,
        };
        println!(
            "{}",
            super::semantic_read_json::render_read_json(&project_root, &read_options, &rendered)?
        );
    } else if options.json && options.view == "owner" && options.item_query.is_some() {
        let json_options = options.semantic_query_json_options()?;
        println!(
            "{}",
            render_query_json(&project_root, &json_options, &rendered)?
        );
    } else if options.json {
        let json_options = options.semantic_json_options();
        println!(
            "{}",
            render_search_json(&project_root, &json_options, &rendered)?
        );
    } else {
        print!("{rendered}");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(not(feature = "search"))]
fn run_query_view(_options: &SearchOptions) -> Result<ExitCode, String> {
    Err("query command requires the `search` feature".to_string())
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
    pub(super) fzf_args: Vec<String>,
    pub(super) item_query: Option<String>,
    pub(super) read_selector: Option<String>,
    pub(super) item_names_only: bool,
    pub(super) item_code: bool,
    pub(super) item_projection_metadata: bool,
    pub(super) source_version: QuerySourceVersion,
    pub(super) workspace: bool,
    paths: Vec<PathBuf>,
}

impl SearchOptions {
    fn from_query(options: QuerySearchOptions) -> Self {
        Self {
            view: options.view,
            query: options.query,
            json: options.json,
            output_view: options.output_view,
            package: options.package,
            seeds: options.seeds,
            pipes: options.pipes,
            query_set: options.query_set,
            item_query: options.item_query,
            read_selector: options.read_selector,
            item_names_only: options.item_names_only,
            item_code: options.item_code,
            source_version: options.source_version,
            workspace: options.workspace,
            paths: options.paths,
            ..Self::default()
        }
    }

    fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Self, String> {
        let mut args = args.into_iter();
        let mut options = Self::default();
        let mut positional_only = false;
        let mut pending_option: Option<String> = None;
        let mut positionals = Vec::<std::ffi::OsString>::new();
        while let Some(arg) = args.next() {
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
                    "--fzf-arg" => options.fzf_args.push(value.to_string()),
                    "--query" => options.item_query = Some(value.to_string()),
                    _ => unreachable!("known pending search option"),
                }
                continue;
            }
            if positional_only {
                positionals.push(arg);
                continue;
            }
            if !matches!(arg.to_str(), Some("--query" | "--query-set"))
                && options.query_set.is_empty()
                && options.item_query.is_none()
                && positionals.len() == 1
                && positionals
                    .first()
                    .and_then(|view| view.to_str())
                    .is_some_and(search_view_requires_query)
            {
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
                "--names-only" => options.item_names_only = true,
                "--code" => options.item_code = true,
                "--trace" => options.trace = true,
                "--explain" => options.explain = true,
                "--item-slice" => options.pipes.push("items".to_string()),
                "--package" | "--owner" | "--dependency" | "--scope" | "--view" | "--depth"
                | "--dir" | "--edge" | "--per-owner" | "--seeds" | "--owners" | "--hits"
                | "--query-set" | "--query" | "--fzf-arg" => {
                    pending_option = Some(value.to_string())
                }
                "--fzf" => {
                    options
                        .fzf_args
                        .extend(args.by_ref().map(|arg| arg.to_string_lossy().to_string()));
                    break;
                }
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
        if options.item_names_only && options.item_code {
            return Err("search --names-only and --code cannot be combined".to_string());
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
        if !is_known_search_view(&self.view) {
            return Ok(());
        }
        if self.view == "fzf" && self.query_set.is_empty() && self.query.is_none() {
            self.query = self.item_query.take();
        }
        if search_view_requires_query(&self.view) {
            if !self.query_set.is_empty() {
                self.query = Some(self.query_set.join(","));
            } else if self.query.is_none() {
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
        let mut saw_project_root = false;
        for value in values {
            if is_search_pipe(&value) {
                self.pipes.push(value);
            } else if is_explicit_rust_project_root(&value) {
                saw_project_root = true;
                self.paths.push(PathBuf::from(value));
            } else if saw_project_root {
                return Err("expected at most one PROJECT_ROOT argument".to_string());
            } else {
                self.push_scope(value);
            }
        }
        Ok(())
    }

    fn push_scope(&mut self, value: String) {
        if let Some(scope) = &mut self.scope {
            scope.push(',');
            scope.push_str(&value);
        } else {
            self.scope = Some(value);
        }
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
            fzf_args: self.fzf_args.clone(),
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
    fn semantic_query_json_options(&self) -> Result<SemanticQueryJsonOptions, String> {
        let Some(query) = self.item_query.clone() else {
            return Err("query JSON requires an owner item query".to_string());
        };
        Ok(SemanticQueryJsonOptions {
            query,
            item_names_only: self.item_names_only,
        })
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
            fzf_args: self.fzf_args.clone(),
            item_query: self.item_query.clone(),
            item_names_only: self.item_names_only,
            item_code: self.item_code,
            item_projection_metadata: self.item_projection_metadata,
        }
    }

    #[cfg(feature = "search")]
    fn project_root(&self) -> Result<PathBuf, String> {
        if self.workspace {
            return match self.paths.as_slice() {
                [path] => Ok(path.clone()),
                [] => env::current_dir()
                    .map_err(|error| format!("failed to read current dir: {error}")),
                _ => unreachable!("parse enforces at most one path"),
            };
        }
        match self.paths.as_slice() {
            [path] => rust_project_root_for_path(path),
            [] => discover_rust_project_root(),
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
            [path] => rust_project_root_for_path(path),
            [] => {
                let current = env::current_dir()
                    .map_err(|error| format!("failed to read current dir: {error}"))?;
                rust_project_root_for_path(&current)
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
                            "rs-harness no longer writes Codex hook configs; use asp hook install --client codex".to_string(),
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
                "guide" if options.command == "doctor" => {
                    return Err("rs-harness agent guide moved to rs-harness guide".to_string());
                }
                "install" | "doctor" | "hook" | "guard" if options.command == "doctor" => {
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

fn validate_search_options(options: &SearchOptions) -> Result<(), String> {
    if !is_known_search_view(&options.view) {
        return Err(format!("unknown search view: {}", options.view));
    }
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
    if !options.fzf_args.is_empty() {
        if options.view != "fzf" {
            return Err("search fzf options are only supported by search fzf".to_string());
        }
        for arg in &options.fzf_args {
            validate_fzf_arg(arg)?;
        }
    }
    Ok(())
}

fn validate_fzf_arg(value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err("empty fzf option is not supported for agent search".to_string());
    }
    if value.chars().any(char::is_whitespace) {
        return Err(format!("unsupported fzf option for agent search: {value}"));
    }
    if matches!(value, "--exact" | "-e" | "-i" | "+i") {
        return Ok(());
    }
    if value.starts_with("--scheme=") {
        let scheme = value.trim_start_matches("--scheme=");
        if matches!(scheme, "default" | "path" | "history") {
            return Ok(());
        }
    }
    Err(format!("unsupported fzf option for agent search: {value}"))
}
