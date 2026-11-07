//! The `key` service's cases, on values alone.

use super::service::{hash_key, hash_row, mix, word_of};
use super::types::{HashWord, KeyCellsHash, On};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::{Flag, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[derive(Clone, Copy)]
enum Scenario {
    AbsentCell,
    NegativeNumber,
    RaisedFlag,
    WholeAgreesWithAllColumns,
    MixOfOneIntoZero,
}
type Input = Scenario;
type Value = u64;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_absent_cell_hashes_as_the_zero_word"),
        word: Word::Succeeds,
        input: || Scenario::AbsentCell,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_negative_number_is_its_twos_complement_word"),
        word: Word::Succeeds,
        input: || Scenario::NegativeNumber,
        expect: Expect::Value(u64::MAX),
    },
    Case {
        name: CaseName("a_raised_flag_is_the_word_one"),
        word: Word::Succeeds,
        input: || Scenario::RaisedFlag,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_whole_row_hashes_as_all_of_its_columns"),
        word: Word::Succeeds,
        input: || Scenario::WholeAgreesWithAllColumns,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("mixing_one_into_zero_is_the_standard_fx_hash_of_one"),
        word: Word::Succeeds,
        input: || Scenario::MixOfOneIntoZero,
        expect: Expect::Value(1),
    },
];

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    Ok(match input {
        Scenario::AbsentCell => hash_key(std::iter::once(None)).0,
        Scenario::NegativeNumber => word_of(&Cell::Number(Integer(-1))).0,
        Scenario::RaisedFlag => word_of(&Cell::Flag(Flag::Raised)).0,
        Scenario::WholeAgreesWithAllColumns => {
            let row = [Cell::Number(Integer(4)), Cell::Flag(Flag::Raised)];
            let columns = [ColumnIndex(0), ColumnIndex(1)];
            u64::from(hash_row(&row, On::Whole) == hash_row(&row, On::Columns(&columns)))
        }
        Scenario::MixOfOneIntoZero => {
            let mut standard = rustc_hash::FxHasher::default();
            std::hash::Hasher::write_u64(&mut standard, 1);
            u64::from(mix(KeyCellsHash(0), HashWord(1)).0 == std::hash::Hasher::finish(&standard))
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
