//! The `fixed_names` service's cases: names each stated once are checked as given, and so is no name; a name given twice and an
//! empty name are refused, with their places.

use super::errors::FixedNamesError;
use super::service::checked;
use super::types::FixedWord;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::terms::types::NameText;

type Input = &'static [NameText<'static>];
/// How many names were checked, and whether they are the names given, in their order.
type Value = (usize, bool);
type Failure = FixedNamesError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("fixed_names_each_stated_once_are_checked_as_given"),
        word: Word::Succeeds,
        input: || &[NameText("next-token"), NameText("matching-delimiter")],
        expect: Expect::Value((2, true)),
    },
    Case {
        name: CaseName("no_fixed_name_is_checked"),
        word: Word::Succeeds,
        input: || &[],
        expect: Expect::Value((0, true)),
    },
    Case {
        name: CaseName("a_fixed_name_given_twice_is_refused"),
        word: Word::Refuses,
        input: || {
            &[
                NameText("next-token"),
                NameText("matching-delimiter"),
                NameText("next-token"),
            ]
        },
        expect: Expect::Error(FixedNamesError::Twice {
            first: FixedWord(0),
            again: FixedWord(2),
        }),
    },
    Case {
        name: CaseName("an_empty_fixed_name_is_refused"),
        word: Word::Refuses,
        input: || {
            &[
                NameText("next-token"),
                NameText("matching-delimiter"),
                NameText(""),
            ]
        },
        expect: Expect::Error(FixedNamesError::Empty {
            place: FixedWord(2),
        }),
    },
];

fn subject(names: Input) -> Result<Value, Failure> {
    let passed = checked(names)?;
    Ok((passed.0.len(), passed.0 == names))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
