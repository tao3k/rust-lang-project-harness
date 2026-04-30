//! Command-line entry point for the Rust project harness.

use std::process::ExitCode;

fn main() -> ExitCode {
    xiuxian_harness_rust_lang_project::run_cli_from_env()
}
