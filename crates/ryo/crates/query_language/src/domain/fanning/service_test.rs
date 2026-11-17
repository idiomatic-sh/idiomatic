//! A join's fan is what its keys are proven to pair: as written for an `all-pairs join`; one row at most when a side is distinct
//! on its keys; every row of the file when its keys are the file alone; rows that share a key otherwise.

use super::service::key_fan;
use super::types::JoinKeys;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::distinctness::types::DistinctMemo;
use crate::domain::lineage::types::LineageMemo;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as Reads, KeyFan};

/// The join a row asks of: the right relation, the column of each side it is keyed on, and the fan the query wrote.
#[derive(Clone, Copy, Debug)]
struct Given {
    /// The right side.
    right: Relation,
    /// The left key column of `function`.
    left_on: u32,
    /// The right key column.
    right_on: u32,
    /// The fan as written.
    written: KeyFan,
}

type Input = Given;
type Value = KeyFan;
type Failure = std::convert::Infallible;

/// `id` (column 0) and `file` (column 2) of `function`.
const ID: u32 = 0;
const FILE: u32 = 2;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_all_pairs_join_is_as_written"),
        word: Word::Succeeds,
        input: || Given {
            right: Relation::Function,
            left_on: FILE,
            right_on: FILE,
            written: KeyFan::AllPairs,
        },
        expect: Expect::Value(KeyFan::AllPairs),
    },
    Case {
        name: CaseName("a_join_on_a_key_of_its_right_side_is_unique"),
        word: Word::Succeeds,
        input: || Given {
            right: Relation::Function,
            left_on: ID,
            right_on: ID,
            written: KeyFan::Shared,
        },
        expect: Expect::Value(KeyFan::Unique),
    },
    Case {
        name: CaseName("a_join_on_the_file_alone_is_file_only"),
        word: Word::Succeeds,
        input: || Given {
            right: Relation::Function,
            left_on: FILE,
            right_on: FILE,
            written: KeyFan::Shared,
        },
        expect: Expect::Value(KeyFan::FileOnly),
    },
    Case {
        name: CaseName("a_join_on_a_column_rows_share_is_shared"),
        word: Word::Succeeds,
        input: || Given {
            right: Relation::Function,
            left_on: 1,
            right_on: 1,
            written: KeyFan::Shared,
        },
        expect: Expect::Value(KeyFan::Shared),
    },
];

fn subject(given: Input) -> Result<Value, Failure> {
    Ok(key_fan(
        &[],
        &mut DistinctMemo::default(),
        &mut LineageMemo::default(),
        JoinKeys {
            left: Reads::Relation(Relation::Function),
            left_on: &[ColumnIndex(given.left_on)],
            right: Reads::Relation(given.right),
            right_on: &[ColumnIndex(given.right_on)],
            view: &[],
            written: given.written,
        },
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
