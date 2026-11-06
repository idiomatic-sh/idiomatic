//! The typed row retains the configured version components.

use super::super::interface::RelationsItem as _;
use super::item::{LanguageTarget, LanguageTargetRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Integer};

type Input = ();
type Value = [Bits; 3];
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("target version belongs to its source file"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(7), Bits(3), Bits(11)]),
}];

fn subject(_: Input) -> Result<Value, Failure> {
    Ok(LanguageTarget::words(LanguageTargetRow {
        file: FileId(7),
        major: Integer(3),
        minor: Integer(11),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
