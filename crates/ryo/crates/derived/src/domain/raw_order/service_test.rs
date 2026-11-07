//! The `raw_order` service's cases, on values alone.

use super::service::build;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::raw_words::types::{KeepFlags, KeyStride, OrderPlace, RawWords};
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    DuplicatesKeepOriginalOrder,
    HitsOfAPresentKey,
    HitsOfAReversedKey,
    FirstsSkipDuplicates,
    RecycledOrdering,
    RecycledProbe,
    ZeroStrideIsEmpty,
    RecycledCapacity,
}
type Input = Scenario;
type Value = Cow<'static, [usize]>;
type Failure = ();

const WORDS: [u64; 8] = [9, 4, 1, 7, 9, 4, 0, 8];

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("raw_words_order_duplicates_without_cluster_passthrough"),
        word: Word::Succeeds,
        input: || Scenario::DuplicatesKeepOriginalOrder,
        expect: Expect::Value(Cow::Borrowed(&[3, 1, 0, 2])),
    },
    Case {
        name: CaseName("hits_of_a_present_key_are_its_run"),
        word: Word::Succeeds,
        input: || Scenario::HitsOfAPresentKey,
        expect: Expect::Value(Cow::Borrowed(&[2, 4])),
    },
    Case {
        name: CaseName("hits_of_a_reversed_key_are_empty"),
        word: Word::Succeeds,
        input: || Scenario::HitsOfAReversedKey,
        expect: Expect::Value(Cow::Borrowed(&[2, 2])),
    },
    Case {
        name: CaseName("firsts_keep_one_row_of_each_key"),
        word: Word::Succeeds,
        input: || Scenario::FirstsSkipDuplicates,
        expect: Expect::Value(Cow::Borrowed(&[1, 1, 0, 1])),
    },
    Case {
        name: CaseName("recycling_changes_keys"),
        word: Word::Succeeds,
        input: || Scenario::RecycledOrdering,
        expect: Expect::Value(Cow::Borrowed(&[1, 0])),
    },
    Case {
        name: CaseName("a_recycled_ordering_no_longer_hits_old_keys"),
        word: Word::Succeeds,
        input: || Scenario::RecycledProbe,
        expect: Expect::Value(Cow::Borrowed(&[2, 2])),
    },
    Case {
        name: CaseName("a_zero_stride_stays_empty"),
        word: Word::Succeeds,
        input: || Scenario::ZeroStrideIsEmpty,
        expect: Expect::Value(Cow::Borrowed(&[0, 0])),
    },
    Case {
        name: CaseName("recycling_keeps_the_buffers"),
        word: Word::Succeeds,
        input: || Scenario::RecycledCapacity,
        expect: Expect::Value(Cow::Borrowed(&[1])),
    },
];


/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let order = build(RawWords(&WORDS), KeyStride(2), None);
    let span = |hit: crate::domain::raw_words::types::RowRun| {
        let OrderPlace(start) = hit.start;
        let OrderPlace(end) = hit.end;
        vec![start, end]
    };
    let rows = |order: &super::types::raw_order::value::RawOrder| {
        order.rows().iter().map(|row| row.0).collect::<Vec<_>>()
    };
    Ok(Cow::Owned(match input {
        Scenario::DuplicatesKeepOriginalOrder => rows(&order),
        Scenario::HitsOfAPresentKey => span(order.hits(RawWords(&[9, 4]))),
        Scenario::HitsOfAReversedKey => span(order.hits(RawWords(&[4, 9]))),
        Scenario::FirstsSkipDuplicates => {
            let mut keep = KeepFlags::default();
            order.firsts(&mut keep);
            keep.0.iter().map(|mark| usize::from(*mark)).collect()
        }
        Scenario::RecycledOrdering => {
            let old = build(RawWords(&[u64::MAX, 0, u64::MAX]), KeyStride(1), None);
            rows(&build(RawWords(&[7, 2]), KeyStride(1), Some(old)))
        }
        Scenario::RecycledProbe => {
            let old = build(RawWords(&[u64::MAX, 0, u64::MAX]), KeyStride(1), None);
            span(build(RawWords(&[7, 2]), KeyStride(1), Some(old)).hits(RawWords(&[u64::MAX])))
        }
        Scenario::ZeroStrideIsEmpty => {
            let empty = build(RawWords(&[1, 2]), KeyStride(0), None);
            let mut place = span(empty.hits(RawWords(&[])));
            place.extend(rows(&empty));
            place
        }
        Scenario::RecycledCapacity => {
            let old = build(RawWords(&[u64::MAX, 0, u64::MAX]), KeyStride(1), None);
            let capacities = old.capacities();
            vec![usize::from(build(RawWords(&[7, 2]), KeyStride(1), Some(old)).capacities() == capacities)]
        }
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
