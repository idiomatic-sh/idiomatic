//! The concrete local implementation executes the complete shared contract.
use super::adapter::SuiteLocalAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::fixtures::contract::check;
type Input = ();
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("concrete implementation answers the shared contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    check(&SuiteLocalAdapter);
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    use super::adapter::verify_cases;
    use crate::outbound::{
        execution::fixtures::fake::ExecutionFake, linter::fixtures::fake::LinterFake,
    };
    for empty_rule in [false, true] {
        let rules = tempfile::tempdir().expect("rules");
        let scratch = tempfile::tempdir().expect("scratch");
        if empty_rule {
            let directory = rules.path().join("fixture");
            std::fs::create_dir(&directory).expect("rule");
            std::fs::write(
                directory.join("query.ryoql"),
                "let rows = select * from comment; emit rows subject file key(span);",
            )
            .expect("query");
            std::fs::write(directory.join("cases.toml"), "cases = []").expect("cases");
        }
        let result = verify_cases(
            &LinterFake::default(),
            &ExecutionFake {
                status: 0,
                ..Default::default()
            },
            std::path::Path::new("unreachable"),
            rules.path(),
            scratch.path(),
        );
        let actual = result
            .expect_err("no cases cannot certify correctness")
            .to_string();
        let expected = if empty_rule {
            "a rule requires positive and negative cases"
        } else {
            "0 cases checked; failures:\nno executable rule cases"
        };
        assert_eq!(actual, expected);
    }
}
