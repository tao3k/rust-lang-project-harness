//! Public harness mounting macros.

/// Mount the default Rust project harness into a Cargo test target.
#[macro_export]
macro_rules! rust_project_harness_gate {
    () => {
        #[test]
        fn enforce_rust_project_harness_gate() {
            $crate::assert_rust_project_harness_clean(std::path::Path::new(env!(
                "CARGO_MANIFEST_DIR"
            )));
        }
    };
}

/// Mount an external source-backed harness file from `src/lib.rs` or `src/main.rs`.
#[macro_export]
macro_rules! rust_project_harness_source_gate {
    ($path:literal) => {
        #[cfg(test)]
        #[path = $path]
        mod rust_project_harness_gate;
    };
}
