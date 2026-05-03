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
/// rust_lang_project_harness::rust_project_harness_cargo_test_gate!(config = {
///     rust_lang_project_harness::default_rust_harness_config()
///         .with_verification_profile_hint(
///             rust_lang_project_harness::RustVerificationProfileHint::new(
///                 "src/lib.rs",
///                 [rust_lang_project_harness::RustOwnerResponsibility::PublicApi],
///             ),
///         )
/// });
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
    (config = $config:expr) => {
        #[test]
        fn enforce_rust_project_harness_gate() {
            let config = $config;
            $crate::assert_rust_project_harness_clean_with_config(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
                &config,
            );
        }
    };
    ($config:expr) => {
        $crate::rust_project_harness_cargo_test_gate!(config = $config);
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
