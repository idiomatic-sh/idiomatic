//! The `table_words` service's cases, on values alone.

use ryo_relations::domain::relations::types::{
    CellKind, FileId, Offset, Span, Value as Cell, Words,
};

use super::service::{decode_combined, encode_table, pack, unpacked};
use super::types::WordSlice;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::Table;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Writes a table twice as words and reads it back, as two tables and as one.
    Words,
    /// Reads words that stop short of the end of the second table.
    Cut,
}

/// What a row of the cases does.
type Input = Scenario;
/// How many rows were read back, or whether the check held.
type Value = usize;
type Failure = ();

fn rows() -> Table {
    let mut table = Table {
        kinds: vec![CellKind::File, CellKind::Span],
        ..Table::default()
    };
    for (file, start, end) in [(0, 0, 4), (1, 13, 16)] {
        push_row(
            &mut table,
            &[
                Cell::File(FileId(file)),
                Cell::Span(Span {
                    start: Offset(start),
                    end: Offset(end),
                }),
            ],
        );
    }
    table
}

/// The table written twice.
fn twice() -> Words {
    let mut table_words = Words::default();
    encode_table(&rows(), &[], &mut table_words);
    encode_table(&rows(), &[], &mut table_words);
    table_words
}

fn subject(scenario: Input) -> Value {
    let table = rows();
    let table_words = twice();
    match scenario {
        Scenario::Words => {
            // The tables read as one are the rows of each, one after the other.
            let mut whole = rows();
            for row in table.cells.chunks(table.kinds.len()) {
                push_row(&mut whole, row);
            }
            // Packed, the words are as they were, and they read as the words they pack.
            let packed = pack(WordSlice(&table_words.0));
            let combined = decode_combined(&packed, &mut Vec::new());
            usize::from(
                unpacked(&packed)
                    .map(|each| each.0)
                    .eq(table_words.0.iter().copied())
                    && combined.cells == whole.cells
                    && combined.count == whole.count
                    && combined.kinds == whole.kinds,
            )
        }
        Scenario::Cut => {
            // A table that stops short is left out, with those after it.
            let cut = decode_combined(
                &pack(WordSlice(&table_words.0[..table_words.0.len() - 1])),
                &mut Vec::new(),
            );
            cut.count.0
        }
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("tables_survive_being_words"),
        word: Word::Succeeds,
        input: || Scenario::Words,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_table_that_stops_short_is_left_out_with_those_after_it"),
        word: Word::Succeeds,
        input: || Scenario::Cut,
        expect: Expect::Value(2),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}

#[test]
fn exact_size_packing_matches_legacy_varints_and_retains_no_capacity_slack() {
    use super::service::{encoded_size, with_encoded_word};
    fn legacy(words: &[u64]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for word in words {
            let mut value = *word;
            while value >= 128 {
                bytes.push((value & 127) as u8 | 128);
                value >>= 7;
            }
            bytes.push(value as u8);
        }
        bytes
    }
    for words in [
        vec![],
        vec![0],
        vec![127],
        vec![128],
        vec![16383],
        vec![16384],
        vec![u64::MAX],
        vec![0, 127, 128, 16383, 16384, u64::MAX],
        twice().0,
    ] {
        let packed = pack(WordSlice(&words));
        assert_eq!(packed.0, legacy(&words));
        assert_eq!(encoded_size(WordSlice(&words)), Some(packed.0.len()));
        assert_eq!(
            packed.0.capacity(),
            packed.0.len(),
            "no guessed allocation ratio or shrink-copy"
        );
        assert!(unpacked(&packed).map(|word| word.0).eq(words));
    }
    assert_eq!(with_encoded_word(usize::MAX, 0), None);
    assert_eq!(with_encoded_word(usize::MAX - 9, u64::MAX), None);
    assert_eq!(
        with_encoded_word(usize::MAX - 10, u64::MAX),
        Some(usize::MAX)
    );
}
