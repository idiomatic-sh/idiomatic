//! The raw ordering answers the places of a key's run and the first row of each key.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::raw_order::service::build;
use crate::domain::raw_words::types::{KeepFlags, KeyStride, RawWords};
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    RunOfARepeatedKey,
    FirstOfEachKey,
    ABufferSizedForTheWords,
}
type Input = Scenario;
type Value = Cow<'static, [usize]>;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_repeated_key_has_one_run_of_two_places"),
        word: Word::Succeeds,
        input: || Scenario::RunOfARepeatedKey,
        expect: Expect::Value(Cow::Borrowed(&[0, 2])),
    },
    Case {
        name: CaseName("firsts_mark_the_first_row_of_each_key"),
        word: Word::Succeeds,
        input: || Scenario::FirstOfEachKey,
        expect: Expect::Value(Cow::Borrowed(&[1, 0, 1])),
    },
    Case {
        name: CaseName("capacities_cover_the_words_and_the_rows"),
        word: Word::Succeeds,
        input: || Scenario::ABufferSizedForTheWords,
        expect: Expect::Value(Cow::Borrowed(&[1])),
    },
];

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let order = build(RawWords(&[1, 1, 2]), KeyStride(1), None);
    Ok(Cow::Owned(match input {
        Scenario::RunOfARepeatedKey => {
            let hit = order.hits(RawWords(&[1]));
            vec![hit.start.0, hit.end.0]
        }
        Scenario::FirstOfEachKey => {
            let mut keep = KeepFlags::default();
            order.firsts(&mut keep);
            keep.0.iter().map(|mark| usize::from(*mark)).collect()
        }
        Scenario::ABufferSizedForTheWords => {
            let capacities = order.capacities();
            vec![usize::from(capacities.keys.0 >= 3 && capacities.order.0 >= 3)]
        }
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
