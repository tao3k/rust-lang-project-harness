//! Command-line execution for the Rust project harness binary.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::{
    render_rust_project_harness, render_rust_project_harness_json, run_rust_project_harness,
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
    let options = CliOptions::parse(args)?;
    if options.help {
        print_help();
        return Ok(ExitCode::SUCCESS);
    }
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

#[derive(Debug, Default)]
struct CliOptions {
    json: bool,
    help: bool,
    paths: Vec<PathBuf>,
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
        "rust-project-harness [--json] [PROJECT_ROOT]\n\n\
         Runs the default package-level Rust harness.\n\n\
         Compact text is the default output for humans and repair-oriented agents.\n\
         Use --json to emit the structured RustHarnessReport JSON shape."
    );
}
