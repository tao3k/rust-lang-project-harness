//! Command-line entry point for `rs-harness`.

use std::process::ExitCode;

fn main() -> ExitCode {
    rust_lang_project_harness::run_cli_from_env()
}
