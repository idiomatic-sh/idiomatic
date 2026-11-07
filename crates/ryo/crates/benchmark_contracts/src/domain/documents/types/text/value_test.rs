//! A document's yes or no and digest declarations keep the answer they were given.
use crate::cases::{Case, CaseName, Expect, Word, run};
use super::value::{FileMetadataDigest, ProjectInputsDigest, Sha256Text, YesNo};

#[derive(Clone, Copy)]
enum Scenario {
    TrueIsYes,
    FalseIsNo,
    SomeDigestDeclares,
    NoDigestUndeclares,
}
type Input = Scenario;
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("true is recorded as yes"),
        word: Word::Succeeds,
        input: || Scenario::TrueIsYes,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("false is recorded as no"),
        word: Word::Succeeds,
        input: || Scenario::FalseIsNo,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a present digest is declared in both declarations"),
        word: Word::Succeeds,
        input: || Scenario::SomeDigestDeclares,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an absent digest is undeclared in both declarations"),
        word: Word::Succeeds,
        input: || Scenario::NoDigestUndeclares,
        expect: Expect::Value(true),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    let digest = || Some(Sha256Text("a".repeat(64)));
    Ok(match scenario {
        Scenario::TrueIsYes => YesNo::from(true) == YesNo::Yes,
        Scenario::FalseIsNo => YesNo::from(false) == YesNo::No,
        Scenario::SomeDigestDeclares => {
            FileMetadataDigest::from(digest()) == FileMetadataDigest::Declared(Sha256Text("a".repeat(64)))
                && ProjectInputsDigest::from(digest())
                    == ProjectInputsDigest::Declared(Sha256Text("a".repeat(64)))
        }
        Scenario::NoDigestUndeclares => {
            FileMetadataDigest::from(None) == FileMetadataDigest::Undeclared
                && ProjectInputsDigest::from(None) == ProjectInputsDigest::Undeclared
        }
    })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
