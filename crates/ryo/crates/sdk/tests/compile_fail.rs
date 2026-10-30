//! The UI runner: every pass case under `tests/ui` compiles, then every compile-fail case fails with the errors it records.

#[test]
fn ui() {
    let cases = ::trybuild::TestCases::new();
    cases.pass("tests/ui/pass_*.rs");
    cases.compile_fail("tests/ui/fail_*.rs");
}
