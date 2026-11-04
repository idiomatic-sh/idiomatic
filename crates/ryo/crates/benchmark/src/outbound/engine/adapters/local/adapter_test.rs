//! The concrete local implementation executes the complete shared contract.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::engine::fixtures::contract::check;
type Input = ();
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("concrete implementation answers the shared contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
#[expect(
    clippy::unnecessary_wraps,
    reason = "the shared case runner requires a Result-returning subject for every outcome word"
)]
fn subject(_: Input) -> Result<Value, Failure> {
    check(&crate::outbound::engine::factory::local(
        ryo::linter::Languages::default(),
        None,
    ));
    #[cfg(unix)]
    frontend_scope_uses_literal_missing_paths_not_glob_patterns();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[cfg(unix)]
fn frontend_scope_uses_literal_missing_paths_not_glob_patterns() {
    let missing = r"pkg/missing[1]*?{x}é\name.py";
    let readable = r"pkg/missing1anythingQxé\name.py";
    let exclusions =
        super::adapter::frontend_exclusions(&[ryo::linter::RelativePath(missing.into())].into())
            .unwrap();
    assert_eq!(
        exclusions.len(),
        1,
        "neither readable nor unrequested paths are exempted"
    );
    let matcher = globset::GlobBuilder::new(exclusions[0].as_ref())
        .literal_separator(true)
        .build()
        .unwrap()
        .compile_matcher();
    assert!(matcher.is_match(missing));
    assert!(!matcher.is_match(readable));
    assert!(!matcher.is_match("pkg/missing1anythingQxéname.py"));
}
