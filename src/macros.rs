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

/// Mount the default Rust project harness inside `src/lib.rs` for Cargo tests.
///
/// Use this from downstream crates through a dev-dependency:
///
/// ```rust,ignore
/// #[cfg(test)]
/// rust_lang_project_harness::rust_project_harness_cargo_test_gate!();
/// ```
///
/// The `#[cfg(test)]` guard keeps normal `cargo build` free of the dev-dependency,
/// while `cargo test` and `cargo test --lib` both execute the project harness.
#[macro_export]
macro_rules! rust_project_harness_cargo_test_gate {
    () => {
        mod rust_project_harness_cargo_test_gate {
            #[test]
            fn enforce_rust_project_harness_gate() {
                $crate::assert_rust_project_harness_clean(std::path::Path::new(env!(
                    "CARGO_MANIFEST_DIR"
                )));
            }
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
