use std::env;
use std::path::PathBuf;

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
    pub(super) paths: Vec<PathBuf>,
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
    pub(super) json: bool,
    pub(super) help: bool,
    pub(super) output_view: Option<String>,
    pub(super) package: Option<String>,
    pub(super) seeds: Option<usize>,
    pub(super) paths: Vec<PathBuf>,
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
                | "--package" | "--view" | "--seeds" => {
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
        if options.paths.len() > 1 {
            return Err("expected at most one PROJECT_ROOT argument".to_string());
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
        for value in values {
            self.paths.push(PathBuf::from(value));
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
            ..QuerySearchOptions::default()
        };
        options.paths.push(self.project_root()?);
        if let Some(selector) = self.normalized_selector() {
            if selector_has_glob(&selector) {
                if !self.terms.is_empty() || self.query.is_some() {
                    self.apply_fzf_query(
                        &mut options,
                        self.from_hook.as_deref() == Some("direct-source-read"),
                    )?;
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
            self.apply_fzf_query(&mut options, false)?;
        } else {
            options.view = "prime".to_string();
            options
                .output_view
                .get_or_insert_with(|| "seeds".to_string());
        }
        if self.from_hook.as_deref() == Some("direct-source-read") {
            options.read_selector = self.normalized_selector_preserving_range();
        }
        Ok(options)
    }

    fn apply_fzf_query(
        &self,
        options: &mut QuerySearchOptions,
        normalize_code_shaped_terms: bool,
    ) -> Result<(), String> {
        let terms = if normalize_code_shaped_terms {
            self.code_shaped_fzf_terms()
                .unwrap_or_else(|| self.fzf_terms())
        } else {
            self.fzf_terms()
        };
        if terms.is_empty() {
            return Err("query fzf mode requires --term or --query".to_string());
        }
        options.view = "fzf".to_string();
        options.query = Some(terms.join(","));
        options.query_set = terms;
        options.pipes = self.query_surfaces(&["tests"]);
        options
            .output_view
            .get_or_insert_with(|| "seeds".to_string());
        Ok(())
    }

    fn item_query(&self) -> Option<String> {
        self.query
            .clone()
            .or_else(|| (!self.terms.is_empty()).then(|| self.terms.join("|")))
    }

    fn fzf_terms(&self) -> Vec<String> {
        if !self.terms.is_empty() {
            return self.terms.clone();
        }
        self.query
            .as_deref()
            .map(split_csv_values)
            .unwrap_or_default()
    }

    fn code_shaped_fzf_terms(&self) -> Option<Vec<String>> {
        let raw_terms = self.fzf_terms();
        let mut normalized = Vec::with_capacity(raw_terms.len());
        for term in raw_terms {
            normalized.push(normalize_rust_code_shaped_term(&term)?);
        }
        (!normalized.is_empty()).then_some(normalized)
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

fn normalize_rust_code_shaped_term(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed
            .chars()
            .any(|character| matches!(character, '/' | '\\' | '-'))
    {
        return None;
    }
    let has_syntax_hint = trimmed
        .chars()
        .any(|character| matches!(character, '{' | '}' | '(' | ')' | ':' | '.' | '<' | '>'))
        || trimmed
            .split_whitespace()
            .next()
            .is_some_and(is_rust_query_keyword);
    if !has_syntax_hint {
        return None;
    }

    let mut identifiers = Vec::new();
    let mut current = String::new();
    for character in trimmed.chars() {
        if character == '_' || character.is_ascii_alphanumeric() {
            current.push(character);
        } else if !current.is_empty() {
            identifiers.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        identifiers.push(current);
    }

    identifiers
        .into_iter()
        .filter(|identifier| is_rust_identifier(identifier))
        .rfind(|identifier| !is_rust_query_keyword(identifier))
}

fn is_rust_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn is_rust_query_keyword(value: &str) -> bool {
    matches!(
        value,
        "async"
            | "const"
            | "crate"
            | "enum"
            | "extern"
            | "fn"
            | "impl"
            | "let"
            | "mod"
            | "move"
            | "pub"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "type"
            | "unsafe"
            | "use"
            | "where"
    )
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
