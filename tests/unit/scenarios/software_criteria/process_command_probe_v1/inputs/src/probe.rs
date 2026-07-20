use std::env;
use std::path::PathBuf;
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

pub fn ai_scaffold_probe(config: &ProbeConfig) -> ProbeReceipt {
    let output = execute_probe_command(build_probe_command(config));
    parse_probe_receipt(config, output)
}

fn build_probe_command(config: &ProbeConfig) -> Command {
    let mut command = Command::new(&config.program);
    command
        .current_dir(&config.root)
        .env("PATH", probe_helper_path(config))
        .arg("--print-helper");
    command
}

fn probe_helper_path(config: &ProbeConfig) -> String {
    format!(
        "{}:{}",
        config.helper_bin.parent().unwrap().display(),
        env::var("PATH").unwrap()
    )
}

fn execute_probe_command(mut command: Command) -> Output {
    command.output().expect("probe command should run")
}

fn parse_probe_receipt(config: &ProbeConfig, output: Output) -> ProbeReceipt {
    let text = format!(
        "{}\n{}",
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap()
    );
    let helper = text.split_whitespace().find(|token| token.ends_with("helper"));
    ProbeReceipt {
        program: config.program.clone(),
        root: config.root.clone(),
        helper: helper.map(PathBuf::from),
        command: CommandReceipt {
            status_code: output.status.code(),
            stdout: text.clone(),
            stderr: String::new(),
        },
    }
}
