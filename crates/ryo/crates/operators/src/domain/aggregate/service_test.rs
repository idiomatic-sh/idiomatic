//! Count saturation, digest bit pattern and duplicate group keys are preserved.
use super::service::aggregate;
use crate::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::plan_parts::types::ColumnIndex,
};
type Input = (&'static [ColumnIndex], usize, Option<i64>);
type Value = std::borrow::Cow<'static, [Cell]>;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("count_retains_repeated_key_order"),
        word: Word::Succeeds,
        input: || (&[ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)], 4, None),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[
            Cell::Number(Integer(9)),
            Cell::Number(Integer(7)),
            Cell::Number(Integer(9)),
            Cell::Number(Integer(4)),
        ])),
    },
    Case {
        name: CaseName("digest_precedes_count"),
        word: Word::Succeeds,
        input: || (&[], 3, Some(-5)),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[
            Cell::Number(Integer(-5)),
            Cell::Number(Integer(3)),
        ])),
    },
    Case {
        name: CaseName("count_saturates_at_signed_maximum"),
        word: Word::Succeeds,
        input: || (&[], usize::MAX, None),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[Cell::Number(Integer(
            if usize::BITS < 64 {
                usize::MAX as i64
            } else {
                i64::MAX
            },
        ))])),
    },
    Case {
        name: CaseName("first_missing_key_is_reported"),
        word: Word::Refuses,
        input: || (&[ColumnIndex(4), ColumnIndex(3)], 0, None),
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn { index: 4 }),
    },
];
fn subject((keys, count, digest): Input) -> Result<Value, Failure> {
    let mut output = Vec::new();
    aggregate(
        &[Cell::Number(Integer(7)), Cell::Number(Integer(9))],
        keys,
        count,
        digest,
        &mut output,
    )?;
    Ok(std::borrow::Cow::Owned(output))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
