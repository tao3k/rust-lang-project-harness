#[test]
fn development_install_uses_the_dedicated_optimized_runtime_profile() {
    let descriptor: serde_json::Value = serde_json::from_str(include_str!(
        "../../provider/asp-provider-workspace-install.json"
    ))
    .expect("Rust provider workspace descriptor");
    assert_eq!(
        descriptor.pointer("/workspaceArtifact/root"),
        Some(&serde_json::Value::String(
            "languages/asp-rust/target/provider-runtime/asp-rust".to_owned()
        ))
    );

    let justfile = include_str!("../../Justfile");
    let install = justfile
        .split_once("install:\n")
        .expect("Rust provider install recipe")
        .1
        .split_once("\n\n")
        .map_or_else(|| justfile.to_owned(), |(recipe, _)| recipe.to_owned());
    assert!(install.contains("cargo build --offline --profile provider-runtime --features"));
    assert!(!install.contains("target/debug"));
    assert!(!install.contains("CARGO_TARGET_DIR"));
}
