use std::fs;

use tempfile::TempDir;

use crate::cli::support::run_cli;

#[test]
fn cli_query_hook_line_range_code_uses_projection_rows_for_nested_impl() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"nested-impl-window\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write manifest");
    fs::write(
        root.join("src/lib.rs"),
        r#"pub struct LocalNativeCliBackend;

#[derive(Clone)]
struct LocalNativeCommand {
    program: String,
    args: Vec<String>,
}

impl LocalNativeCliBackend {
}

impl LocalNativeCommand {
    fn argv(&self) -> Vec<String> {
        let mut argv = Vec::with_capacity(self.args.len() + 1);
        argv.push(self.program.clone());
        argv.extend(self.args.clone());
        argv
    }
}
"#,
    )
    .expect("write lib");

    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:9:18".as_ref(),
        "--code".as_ref(),
        "--workspace".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(!stdout.contains("[search-owner]"), "{stdout}");
    assert!(!stdout.contains("[read-plan]"), "{stdout}");
    assert_no_punctuation_only_lines(&stdout);
    assert!(
        stdout.starts_with("impl LocalNativeCliBackend {\n}"),
        "{stdout}"
    );
    assert!(stdout.contains("impl LocalNativeCommand {\n"), "{stdout}");
    assert!(
        stdout.contains("        argv.push(self.program.clone());\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("        argv.extend(self.args.clone());\n"),
        "{stdout}"
    );
    assert!(
        !stdout.contains("argv.push(self.program.clone())\n"),
        "{stdout}"
    );
}

fn assert_no_punctuation_only_lines(stdout: &str) {
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "}" {
            continue;
        }
        assert!(
            trimmed.chars().any(|ch| ch.is_alphanumeric() || ch == '_'),
            "punctuation-only compact row leaked: {stdout}"
        );
    }
}
