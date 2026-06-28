use std::env;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeConfig {
    pub program: PathBuf,
    pub root: PathBuf,
    pub helper_bin: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandReceipt {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReceipt {
    pub program: PathBuf,
    pub root: PathBuf,
    pub helper: Option<PathBuf>,
    pub command: CommandReceipt,
}

pub fn structured_probe_receipt(
    config: &ProbeConfig,
    output: io::Result<Output>,
) -> ProbeReceipt {
    let command = command_receipt(output);
    let helper = helper_path_from_receipt(&command);
    ProbeReceipt {
        program: config.program.clone(),
        root: config.root.clone(),
        helper,
        command,
    }
}

pub fn probe_command(config: &ProbeConfig) -> Command {
    let mut command = Command::new(&config.program);
    command.current_dir(&config.root);
    if let Some(parent) = config.helper_bin.parent() {
        command.env("PATH", prepend_path(parent));
    }
    command.arg("--print-helper");
    command
}

fn helper_path_from_receipt(receipt: &CommandReceipt) -> Option<PathBuf> {
    let output = format!("{}\n{}", receipt.stdout, receipt.stderr);
    output
        .split_whitespace()
        .find(|token| token.ends_with("helper"))
        .map(PathBuf::from)
}

fn prepend_path(path: &Path) -> OsString {
    match env::var_os("PATH") {
        Some(current) => {
            let mut paths = Vec::from([path.to_path_buf()]);
            paths.extend(env::split_paths(&current));
            env::join_paths(paths).unwrap_or(current)
        }
        None => path.as_os_str().to_os_string(),
    }
}

fn command_receipt(output: io::Result<Output>) -> CommandReceipt {
    match output {
        Ok(output) => CommandReceipt {
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        },
        Err(error) => CommandReceipt {
            status_code: None,
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
}
