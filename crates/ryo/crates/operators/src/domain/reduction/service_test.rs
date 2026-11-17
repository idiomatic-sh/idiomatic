//! The `reduction` service's cases, on values alone.

use super::service::{folded, hash_of};
use super::types::{DigestDatum, HashScratch};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::WholeNumber;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::RowCount;

/// A scenario: whether what the part answers is what the case states.
type Input = fn() -> bool;
type Value = bool;
type Failure = ();

/// A group digest counts each distinct hash once, whatever their order.
fn folds_distinct_hashes_in_any_order() -> bool {
    let (mut one, mut other) = (vec![3, 1, 3, 2], vec![2, 3, 1]);
    let (first, second) = (
        folded(HashScratch(&mut one)),
        folded(HashScratch(&mut other)),
    );
    first == second && first.counted == RowCount(3)
}

/// The same value hashes alike, and a number differs from its text.
fn hashes_are_stable_per_value() -> bool {
    hash_of(DigestDatum::Number(WholeNumber(7))) == hash_of(DigestDatum::Number(WholeNumber(7)))
        && hash_of(DigestDatum::Text(CellText("a"))) == hash_of(DigestDatum::Text(CellText("a")))
        && hash_of(DigestDatum::Text(CellText("a"))) != hash_of(DigestDatum::Text(CellText("b")))
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_digest_counts_each_distinct_hash_once"),
        word: Word::Succeeds,
        input: || folds_distinct_hashes_in_any_order,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_value_hashes_alike_every_time"),
        word: Word::Succeeds,
        input: || hashes_are_stable_per_value,
        expect: Expect::Value(true),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(input())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
