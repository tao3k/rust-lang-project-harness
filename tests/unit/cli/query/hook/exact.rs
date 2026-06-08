use std::fs;

use tempfile::TempDir;

use crate::cli::support::run_cli;

#[test]
fn cli_query_hook_line_range_code_preserves_exact_line_bytes() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"hook-exact-range\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write manifest");
    fs::write(
        root.join("src/lib.rs"),
        "fn first() {\r\n\tlet value = 1;\r\n}\r\nfn second() {\r\n    let value = 2;\r\n}",
    )
    .expect("write source");

    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:2:5".as_ref(),
        "--code".as_ref(),
        "--workspace".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        output.stdout,
        b"\tlet value = 1;\r\n}\r\nfn second() {\r\n    let value = 2;\r\n"
    );
}
