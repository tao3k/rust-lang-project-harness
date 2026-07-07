use std::env;
use std::path::PathBuf;

use super::query_source::{QuerySourceVersion, parse_query_source_version};

#[derive(Debug, Default)]
pub(super) struct QuerySearchOptions {
    pub(super) view: String,
    pub(super) query: Option<String>,
    pub(super) json: bool,
    pub(super) output_view: Option<String>,
    pub(super) package: Option<String>,
    pub(super) seeds: Option<usize>,
    pub(super) pipes: Vec<String>,
    pub(super) query_set: Vec<String>,
    pub(super) item_query: Option<String>,
    pub(super) read_selector: Option<String>,
    pub(super) item_names_only: bool,
    pub(super) item_code: bool,
    pub(super) source_version: QuerySourceVersion,
    pub(super) workspace_root: Option<PathBuf>,
}

#[derive(Debug, Default)]
pub(super) struct QueryOptions {
    pub(super) selector: Option<String>,
    pub(super) query: Option<String>,
    pub(super) terms: Vec<String>,
    pub(super) surfaces: Vec<String>,
    pub(super) from_hook: Option<String>,
    pub(super) names_only: bool,
    pub(super) code: bool,
    pub(super) workspace: bool,
    pub(super) json: bool,
    pub(super) help: bool,
    pub(super) output_view: Option<String>,
    pub(super) package: Option<String>,
    pub(super) seeds: Option<usize>,
    pub(super) source_version: QuerySourceVersion,
    pub(super) paths: Vec<PathBuf>,
    pub(super) workspace_root: Option<PathBuf>,
}

impl QueryOptions {
    pub(super) fn parse(
        args: impl IntoIterator<Item = std::ffi::OsString>,
    ) -> Result<Self, String> {
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
                    "--selector" => options.set_selector(value)?,
                    "--query" => options.set_query(value)?,
                    "--term" => options.terms.extend(split_csv_values(value)),
                    "--surface" | "--pipe" => options.add_surfaces(value)?,
                    "--from-hook" => options.from_hook = Some(value.to_string()),
                    "--package" => options.package = Some(value.to_string()),
                    "--view" => options.output_view = Some(value.to_string()),
                    "--seeds" => options.seeds = Some(parse_usize_option(&option, value)?),
                    "--source" => options.source_version = parse_query_source_version(value)?,
                    "--workspace" => {
                        if value.starts_with('-') {
                            return Err("--workspace requires a project root".to_string());
                        }
                        options.workspace = true;
                        options.workspace_root = Some(PathBuf::from(value));
                    }
                    _ => unreachable!("known pending query option"),
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
                "--names-only" => options.names_only = true,
                "--code" => options.code = true,
                "--selector" | "--query" | "--term" | "--surface" | "--pipe" | "--from-hook"
                | "--package" | "--view" | "--seeds" | "--source" | "--workspace" => {
                    pending_option = Some(value.to_string());
                }
                value if is_search_pipe(value) => options.surfaces.push(value.to_string()),
                value if value.starts_with('-') => {
                    return Err(format!("unknown query option: {value}"));
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
        if options.names_only && options.code {
            return Err("query --names-only and --code cannot be combined".to_string());
        }
        options.apply_positionals(positionals)?;
        if options.names_only
            && options.selector.is_none()
            && (!options.terms.is_empty() || options.query.is_some())
        {
            return Err(
                "query --names-only requires an owner selector; use `asp rust search lexical '<term>' owner --workspace <workspace-root> --view seeds` for workspace term discovery"
                    .to_string(),
            );
        }
        if options.code && !options.paths.is_empty() {
            return Err(
                "query does not accept positional WORKSPACE; use --workspace <WORKSPACE>"
                    .to_string(),
            );
        }
        if !options.paths.is_empty() {
            return Err(
                "query does not accept positional WORKSPACE; use --workspace <WORKSPACE>"
                    .to_string(),
            );
        }
        if let Some(view) = options.output_view.as_deref()
            && !matches!(view, "graph" | "hits" | "both" | "seeds" | "read-packet")
        {
            return Err(format!("unknown query --view mode: {view}"));
        }
        Ok(options)
    }

    fn apply_positionals(&mut self, positionals: Vec<std::ffi::OsString>) -> Result<(), String> {
        let mut values = positionals
            .into_iter()
            .map(|value| {
                value
                    .into_string()
                    .map_err(|_| "expected UTF-8 query arguments".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        if self.selector.is_none()
            && values
                .first()
                .is_some_and(|value| self.should_treat_as_selector(value))
        {
            self.selector = Some(values.remove(0));
        }
        if !values.is_empty() {
            return Err(
                "query does not accept positional WORKSPACE; use --workspace <WORKSPACE>"
                    .to_string(),
            );
        }
        Ok(())
    }

    fn should_treat_as_selector(&self, value: &str) -> bool {
        if matches!(value, "." | "..") {
            return false;
        }
        if selector_has_glob(value) || has_rust_file_selector(value) || has_selector_prefix(value) {
            return true;
        }
        (self.query.is_some() || !self.terms.is_empty() || self.from_hook.is_some())
            && !PathBuf::from(value).is_dir()
    }

    fn set_selector(&mut self, selector: &str) -> Result<(), String> {
        if self
            .selector
            .as_deref()
            .is_some_and(|existing| existing != selector)
        {
            return Err("expected only one query selector".to_string());
        }
        self.selector = Some(selector.to_string());
        Ok(())
    }

    fn set_query(&mut self, query: &str) -> Result<(), String> {
        if self
            .query
            .as_deref()
            .is_some_and(|existing| existing != query)
        {
            return Err("expected only one query expression".to_string());
        }
        self.query = Some(query.to_string());
        Ok(())
    }

    fn add_surfaces(&mut self, surfaces: &str) -> Result<(), String> {
        for surface in split_csv_values(surfaces) {
            if !is_search_pipe(&surface) {
                return Err(format!("unknown query surface: {surface}"));
            }
            self.surfaces.push(surface);
        }
        Ok(())
    }

    pub(super) fn search_options(&self) -> Result<QuerySearchOptions, String> {
        let mut options = QuerySearchOptions {
            json: self.json,
            output_view: self.output_view.clone(),
            package: self.package.clone(),
            seeds: self.seeds,
            item_names_only: self.names_only,
            item_code: self.code,
            source_version: self.source_version,
            workspace_root: self.workspace_root.clone(),
            ..QuerySearchOptions::default()
        };
        if options.workspace_root.is_none() {
            options.workspace_root = Some(self.project_root()?);
        }
        if let Some(selector) = self.normalized_selector() {
            if selector_has_glob(&selector) {
                if !self.terms.is_empty() || self.query.is_some() {
                    if self.names_only {
                        return Err(
                            "query --names-only requires an owner selector; workspace term discovery is `asp rust search lexical '<term>' owner --workspace <workspace-root> --view seeds`"
                                .to_string(),
                        );
                    }
                    return Err(self.workspace_term_discovery_error());
                } else {
                    options.view = "prime".to_string();
                    options
                        .output_view
                        .get_or_insert_with(|| "seeds".to_string());
                }
            } else {
                options.view = "owner".to_string();
                options.query = Some(selector);
                options.pipes = self.query_surfaces(&["items", "tests"]);
                options.item_query = self.item_query();
                if options.item_query.is_none() {
                    options
                        .output_view
                        .get_or_insert_with(|| "seeds".to_string());
                }
            }
        } else if !self.terms.is_empty() || self.query.is_some() {
            return Err(self.workspace_term_discovery_error());
        } else {
            options.view = "prime".to_string();
            options
                .output_view
                .get_or_insert_with(|| "seeds".to_string());
        }
        if self.from_hook.as_deref() == Some("direct-source-read") || self.code {
            options.read_selector = self.normalized_selector_preserving_range();
        }
        Ok(options)
    }

    fn workspace_term_discovery_error(&self) -> String {
        let term = if !self.terms.is_empty() {
            self.terms.join(" ")
        } else {
            self.query.clone().unwrap_or_else(|| "<term>".to_string())
        };
        format!(
            "query workspace term discovery is owned by ASP search lexical; run `asp rust search lexical '{}' owner tests --workspace <workspace-root> --view seeds`",
            term.replace('\'', "'\\''")
        )
    }

    fn item_query(&self) -> Option<String> {
        self.query
            .clone()
            .or_else(|| (!self.terms.is_empty()).then(|| self.terms.join("|")))
    }

    fn normalized_selector(&self) -> Option<String> {
        self.selector
            .as_deref()
            .map(strip_selector_prefix)
            .map(strip_rust_line_suffix)
            .map(str::trim)
            .filter(|selector| !selector.is_empty())
            .map(ToOwned::to_owned)
    }

    fn normalized_selector_preserving_range(&self) -> Option<String> {
        self.selector
            .as_deref()
            .map(strip_selector_prefix)
            .map(str::trim)
            .filter(|selector| !selector.is_empty())
            .map(ToOwned::to_owned)
    }

    fn query_surfaces(&self, defaults: &[&str]) -> Vec<String> {
        if self.surfaces.is_empty() {
            defaults
                .iter()
                .map(|surface| (*surface).to_string())
                .collect()
        } else {
            self.surfaces.clone()
        }
    }

    fn project_root(&self) -> Result<PathBuf, String> {
        if let Some(path) = self.workspace_root.as_ref() {
            return Ok(path.clone());
        }
        match self.paths.as_slice() {
            [path] => Ok(path.clone()),
            [] => {
                env::current_dir().map_err(|error| format!("failed to read current dir: {error}"))
            }
            _ => unreachable!("parse enforces at most one path"),
        }
    }
}

fn selector_has_glob(value: &str) -> bool {
    value
        .chars()
        .any(|character| matches!(character, '*' | '?' | '[' | ']' | '{' | '}'))
}

fn has_selector_prefix(value: &str) -> bool {
    value
        .split_once(':')
        .is_some_and(|(prefix, _)| matches!(prefix, "owner" | "tests" | "test" | "path"))
}

fn has_rust_file_selector(value: &str) -> bool {
    strip_rust_line_suffix(strip_selector_prefix(value)).ends_with(".rs")
}

fn strip_selector_prefix(value: &str) -> &str {
    value
        .split_once(':')
        .and_then(|(prefix, rest)| {
            matches!(prefix, "owner" | "tests" | "test" | "path").then_some(rest)
        })
        .unwrap_or(value)
}

fn strip_rust_line_suffix(value: &str) -> &str {
    let Some((path_and_start, end_line)) = value.rsplit_once(':') else {
        return value;
    };
    if path_and_start.ends_with(".rs") && end_line.parse::<usize>().is_ok() {
        return path_and_start;
    }
    let Some((path, start_line)) = path_and_start.rsplit_once(':') else {
        return value;
    };
    if path.ends_with(".rs")
        && start_line.parse::<usize>().is_ok()
        && end_line.parse::<usize>().is_ok()
    {
        return path;
    }
    value
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
