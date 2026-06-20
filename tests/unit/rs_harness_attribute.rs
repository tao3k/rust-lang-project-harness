#[rs_harness::test(
    config = rust_lang_project_harness::default_rust_harness_config()
        .with_cargo_test_advice_allow_explanation(
            "scope=rs_harness attribute smoke; owner=rs_harness_attribute test; finding_category=advisory harness smoke findings; why_safe_now=the test verifies the allow_advice branch after the gate runs; cleanup_trigger=remove when the attribute no longer supports allow_advice",
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
            "scope=rs_harness explicit-root smoke; owner=rs_harness_attribute test; finding_category=advisory harness smoke findings; why_safe_now=the test verifies explicit project_root forwarding after the gate runs; cleanup_trigger=remove when explicit project_root no longer needs allow_advice coverage",
        ),
    allow_advice
)]
fn rs_harness_test_attribute_accepts_explicit_project_root() {
    assert!(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).is_dir());
}

#[rs_harness::test(
    config = rust_lang_project_harness::default_rust_harness_config()
        .with_cargo_test_advice_allow_explanation(
            "scope=rs_harness should_panic smoke; owner=rs_harness_attribute test; finding_category=advisory harness smoke findings; why_safe_now=the test verifies libtest should_panic forwarding after the gate runs; cleanup_trigger=remove when should_panic forwarding no longer needs allow_advice coverage",
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
