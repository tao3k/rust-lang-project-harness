use std::env;
use std::path::PathBuf;
use std::process::Command;

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
    let output = Command::new(&config.program)
        .current_dir(&config.root)
        .env(
            "PATH",
            format!(
                "{}:{}",
                config.helper_bin.parent().unwrap().display(),
                env::var("PATH").unwrap()
            ),
        )
        .arg("--print-helper")
        .output()
        .expect("probe command should run");
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
