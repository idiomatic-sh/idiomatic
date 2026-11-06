//! The `table_words` service's cases, on values alone.

use ryo_relations::domain::relations::types::{
    CellKind, FileId, Offset, Span, Value as Cell, Words,
};

use super::service::{decode_combined, decode_complete, encode_table, pack, unpacked};
use super::types::WordSlice;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::rows::service::push_row;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Writes a table twice as words and reads it back, as two tables and as one.
    Words,
    /// Reads words that stop short of the end of the second table.
    Cut,
    /// Asks for a complete read of words that stop short.
    Required,
}

/// What a row of the cases does.
type Input = Scenario;
/// How many rows were read back, or whether the check held.
type Value = usize;
type Failure = super::errors::TableWordsError;

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

fn subject(scenario: Input) -> Result<Value, Failure> {
    let table = rows();
    let table_words = twice();
    Ok(match scenario {
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
        Scenario::Required => {
            return decode_complete(
                &pack(WordSlice(&table_words.0[..table_words.0.len() - 1])),
                &mut Vec::new(),
            )
            .map(|table| table.count.0);
        }
    })
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
    Case {
        name: CaseName("a_required_read_of_words_that_stop_short_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Required,
        expect: Expect::Error(super::errors::TableWordsError::Incomplete),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use crate::domain::rows::types::Table;

#[cfg(test)]
mod packing_test;
