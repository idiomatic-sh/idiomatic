//! The `keyed_ranges` service's cases: a key's rows are found by the key in the file they were filed in, each key once,
//! whether the key is found by position or through the hash.

use super::service::{ask, file, gather, new_slot, next_file};
use super::types::{Asked, KeyedRanges};
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

/// (the keys asked, whether the file is the next one).
type Input = (&'static [u64], bool);
/// 1 when every key was filed else 0, the missing keys, then the gathered rows' two columns one after the other.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases: the dense bound is 100; keys 5 (two rows: 50, 51), 7 (no rows) and 500 (one row: 5000,
/// found through the hash) are filed from one batch.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("filed_keys_answer_their_rows_in_key_order_each_key_once"),
        word: Word::Succeeds,
        input: || (&[500, 5, 5], false),
        expect: Expect::Value(Cow::Borrowed(&[1, 500, 5, 5, 5000, 50, 51])),
    },
    Case {
        name: CaseName("a_key_filed_with_no_rows_is_filed_and_answers_nothing"),
        word: Word::Succeeds,
        input: || (&[7], false),
        expect: Expect::Value(Cow::Borrowed(&[1])),
    },
    Case {
        name: CaseName("a_key_never_asked_is_missing_once"),
        word: Word::Succeeds,
        input: || (&[8, 8, 900], false),
        expect: Expect::Value(Cow::Borrowed(&[0, 8, 900])),
    },
    Case {
        name: CaseName("an_empty_list_asks_and_answers_nothing"),
        word: Word::Succeeds,
        input: || (&[], false),
        expect: Expect::Value(Cow::Borrowed(&[1])),
    },
    Case {
        name: CaseName("the_next_file_has_no_rows_of_the_last"),
        word: Word::Succeeds,
        input: || (&[5, 500], true),
        expect: Expect::Value(Cow::Borrowed(&[0, 5, 500])),
    },
];

fn subject((keys, next): Input) -> Result<Value, Failure> {
    let mut store = KeyedRanges::default();
    next_file(&mut store, 100);
    let slot = new_slot(&mut store, 2);
    let (mut missing, mut asked) = (Vec::new(), Vec::new());
    let _ = ask(&mut store, slot, &[5, 7, 500], &mut missing, &mut asked);
    let _ = file(&mut store, slot, &missing, 0, &[&[500, 5, 5], &[5000, 50, 51]], 3);
    if next {
        next_file(&mut store, 100);
    }
    let mut missing = Vec::new();
    let found = ask(&mut store, slot, keys, &mut missing, &mut asked);
    let mut out = vec![Vec::new(), Vec::new()];
    let _ = gather(&mut store, slot, keys, &mut asked, false, &mut out);
    let mut all = vec![u64::from(found == Asked::Filed)];
    all.extend(missing);
    all.extend(out.concat());
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// The rows of the position cases: the first file (dense bound 400) files keys 0..300, one row each, so the slot keeps its
/// keys by position in the next file (bound 400), where keys 5 (row 50), 399 (row 3990) and 1000 (row 10000, through the
/// hash) are filed.
const DENSE_CASES: &[Case<&'static [u64], Value, Failure>] = &[
    Case {
        name: CaseName("a_slot_that_held_many_keys_finds_them_by_position_in_the_next_file"),
        word: Word::Succeeds,
        input: || &[399, 5, 1000],
        expect: Expect::Value(Cow::Borrowed(&[1, 399, 5, 1000, 3990, 50, 10000])),
    },
    Case {
        name: CaseName("a_key_of_the_last_file_is_absent_by_position_too"),
        word: Word::Succeeds,
        input: || &[6],
        expect: Expect::Value(Cow::Borrowed(&[0, 6])),
    },
];

fn dense_subject(keys: &'static [u64]) -> Result<Value, Failure> {
    let mut store = KeyedRanges::default();
    next_file(&mut store, 400);
    let slot = new_slot(&mut store, 2);
    let first: Vec<u64> = (0..300).collect();
    let tens: Vec<u64> = first.iter().map(|k| k * 10).collect();
    let (mut missing, mut asked) = (Vec::new(), Vec::new());
    let _ = ask(&mut store, slot, &first, &mut missing, &mut asked);
    let _ = file(&mut store, slot, &missing, 0, &[&first, &tens], first.len());
    next_file(&mut store, 400);
    let mut missing = Vec::new();
    let _ = ask(&mut store, slot, &[5, 399, 1000], &mut missing, &mut asked);
    let _ = file(&mut store, slot, &missing, 0, &[&[5, 399, 1000], &[50, 3990, 10000]], 3);
    let mut missing = Vec::new();
    let found = ask(&mut store, slot, keys, &mut missing, &mut asked);
    let mut out = vec![Vec::new(), Vec::new()];
    let _ = gather(&mut store, slot, keys, &mut asked, false, &mut out);
    let mut all = vec![u64::from(found == Asked::Filed)];
    all.extend(missing);
    all.extend(out.concat());
    Ok(Cow::Owned(all))
}

#[test]
fn finds_by_position() {
    run(DENSE_CASES, Word::Succeeds, dense_subject);
}
