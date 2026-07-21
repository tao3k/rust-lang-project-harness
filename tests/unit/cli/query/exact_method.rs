use crate::unit::cli::support::{run_cli, write_search_fixture};
use tempfile::TempDir;

#[test]
fn cli_exact_source_selector_resolves_unique_impl_method() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    for selector in [
        "rust://src/lib.rs#item/method/as_thing",
        "rust://src/lib.rs#item/method/PublicWire::as_thing",
    ] {
        let output = run_cli([
            "query".as_ref(),
            "--selector".as_ref(),
            selector.as_ref(),
            "--workspace".as_ref(),
            root.as_os_str(),
            "--code".as_ref(),
        ]);
        assert!(
            output.status.success(),
            "selector={selector} stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
        assert!(stdout.contains("fn as_thing"), "{stdout}");
        assert!(stdout.contains("domain::make_thing()"), "{stdout}");
        assert!(!stdout.contains("impl WireApi"), "{stdout}");
    }
}
