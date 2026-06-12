#[rs_harness::test(
    config = rust_lang_project_harness::default_rust_harness_config()
        .with_cargo_test_advice_allow_explanation(
            "attribute macro smoke test keeps harness advice visible",
        ),
    allow_advice
)]
fn rs_harness_test_attribute_runs_user_body_after_harness_gate() {
    assert_eq!(2 + 2, 4);
}

#[rs_harness::test(
    project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
    config = rust_lang_project_harness::default_rust_harness_config()
        .with_cargo_test_advice_allow_explanation(
            "attribute macro explicit project root smoke test keeps harness advice visible",
        ),
    allow_advice
)]
fn rs_harness_test_attribute_accepts_explicit_project_root() {
    assert!(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).is_dir());
}

#[rs_harness::test(
    config = rust_lang_project_harness::default_rust_harness_config()
        .with_cargo_test_advice_allow_explanation(
            "attribute macro should_panic smoke test keeps harness advice visible",
        ),
    allow_advice,
    should_panic(expected = "expected panic after harness gate")
)]
fn rs_harness_test_attribute_forwards_should_panic_to_libtest() {
    panic!("expected panic after harness gate");
}

#[rs_harness::test(ignore = "compile-only libtest attribute forwarding smoke")]
fn rs_harness_test_attribute_forwards_ignore_to_libtest() {
    panic!("ignored test body should not run");
}
