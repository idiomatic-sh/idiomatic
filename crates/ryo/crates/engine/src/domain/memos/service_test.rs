//! The `memos` service's cases: an answer is found by its site and every cell of its key, and by nothing less.

use super::service::{asked, clear_memos, keep, row_count, rows};
use super::types::MemoMap;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::MemoSite;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;
use std::borrow::Cow;

/// (the site asked, the key asked, whether the file is the next one).
type Input = (u32, &'static [u64], bool);
/// The rows found and then the cells of the answer, or 99 when no answer is kept.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases: one answer is kept, for site 1 and the key (5, 6), of two rows (7, 8) and (9, 10).
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_answer_is_found_by_its_site_and_its_whole_key"),
        word: Word::Succeeds,
        input: || (1, &[5, 6], false),
        expect: Expect::Value(Cow::Borrowed(&[2, 7, 8, 9, 10])),
    },
    Case {
        name: CaseName("a_key_that_differs_in_its_second_cell_has_no_answer"),
        word: Word::Succeeds,
        input: || (1, &[5, 7], false),
        expect: Expect::Value(Cow::Borrowed(&[99])),
    },
    Case {
        name: CaseName("another_site_with_the_same_key_has_no_answer"),
        word: Word::Succeeds,
        input: || (2, &[5, 6], false),
        expect: Expect::Value(Cow::Borrowed(&[99])),
    },
    Case {
        name: CaseName("a_shorter_key_has_no_answer"),
        word: Word::Succeeds,
        input: || (1, &[5], false),
        expect: Expect::Value(Cow::Borrowed(&[99])),
    },
    Case {
        name: CaseName("the_next_file_has_no_answer_of_the_last"),
        word: Word::Succeeds,
        input: || (1, &[5, 6], true),
        expect: Expect::Value(Cow::Borrowed(&[99])),
    },
];

/// A list of cells.
fn cells(numbers: &[u64]) -> Vec<CellWord> {
    numbers.iter().map(|number| CellWord(*number)).collect()
}

fn subject((site, key, next_file): Input) -> Result<Value, Failure> {
    let mut map = MemoMap::default();
    let _kept = keep(
        &mut map,
        MemoSite(1),
        &cells(&[5, 6]),
        &cells(&[7, 8, 9, 10]),
        Count(2),
    );
    let _other = keep(&mut map, MemoSite(3), &cells(&[5, 7, 1]), &[], Count(0));
    if next_file {
        clear_memos(&mut map);
    }
    Ok(Cow::Owned(match asked(&map, MemoSite(site), &cells(key)) {
        Some(answer) => {
            let mut all = vec![u64::try_from(row_count(&map, answer).0).unwrap_or(u64::MAX)];
            all.extend(rows(&map, answer).iter().map(|cell| cell.0));
            all
        }
        None => vec![99],
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
