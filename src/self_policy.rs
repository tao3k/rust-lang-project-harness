//! Self-applied project harness mount for `cargo test --lib`.

#[cfg(test)]
fn self_apply_harness_config() -> crate::RustHarnessConfig {
    crate::default_rust_harness_config().with_verification_profile_hint(
        crate::RustVerificationProfileHint::new(
            "src/lib.rs",
            [crate::RustOwnerResponsibility::PublicApi],
        )
        .without_verification_tasks()
        .with_rationale("self-apply cargo test gate enforces harness policy; external verification runs through explicit stage configs"),
    )
}

#[cfg(test)]
crate::rust_project_harness_cargo_test_gate!(config = self_apply_harness_config());
