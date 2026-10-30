//! The `table_texts` service's cases, on values alone.

use ryo_language::domain::relations::types::{FileId, Offset, Value as Cell};
use ryo_language::domain::symbols::service::intern;
use ryo_language::domain::symbols::service::{name_count, places_mut};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;

use super::service::datum;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_texts::types::{Datum, SourceView, TextView, Texts};
use ryo_values::domain::plan_parts::types::WholeNumber;
use ryo_values::domain::stored_facts::types::ArenaBytes;

/// A scenario: what a row asks of the part.
type Input = fn() -> Result<Value, TableModelError>;
/// What a scenario answers: a yes as one, or a count.
type Value = usize;
/// Why a scenario is refused.
type Failure = TableModelError;

/// A name that left its file reads as text where its worker's arena holds it, and as its hash where nothing does.
fn a_name_that_left_its_file_reads_from_the_arena() -> Value {
    use ryo_language::domain::relations::types::{NamePlace, Origin, PlaceNumber, WorkerNumber};
    let arenas = vec![ArenaBytes(b"xxhello".to_vec())];
    let place = |origin| NamePlace {
        origin,
        file: FileId(0),
        start: Offset(2),
        end: Offset(7),
    };
    let name = Cell::Name(ryo_language::domain::relations::types::Name {
        hash: ryo_language::domain::relations::types::TextHash(5),
        place: PlaceNumber(0),
    });
    let mut derived = Symbols::default();
    *places_mut(&mut derived) = vec![place(Origin::Derived(WorkerNumber(0)))];
    let mut read = Symbols::default();
    *places_mut(&mut read) = vec![place(Origin::Source)];
    let texts = |symbols, held| Texts {
        symbols,
        source: SourceView(&[]),
        paths: &[],
        arenas: held,
    };
    usize::from(
        datum(name, &texts(&derived, &arenas)) == Datum::Text(TextView("hello"))
            && datum(name, &texts(&derived, &[])) == Datum::Number(WholeNumber(5))
            && datum(name, &texts(&read, &arenas)) == Datum::Number(WholeNumber(5)),
    )
}

const SOURCE: &[u8] = b"mainmain";

fn interning_twice_gives_one_symbol() -> Value {
    let mut symbols = Symbols::default();
    let first = intern(&mut symbols, ByteSlice(SOURCE), Offset(0), Offset(4));
    let again = intern(&mut symbols, ByteSlice(SOURCE), Offset(4), Offset(8));
    usize::from(first == again && name_count(&symbols).0 == 1)
}

fn a_symbol_reads_as_its_text() -> Value {
    let mut symbols = Symbols::default();
    let name = intern(&mut symbols, ByteSlice(SOURCE), Offset(0), Offset(4));
    let texts = Texts {
        symbols: &symbols,
        source: SourceView(SOURCE),
        paths: &[],
        arenas: &[],
    };
    usize::from(datum(Cell::Symbol(name), &texts) == Datum::Text(TextView("main")))
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_name_that_left_its_file_reads_from_the_arena"),
        word: Word::Succeeds,
        input: || || Ok(a_name_that_left_its_file_reads_from_the_arena()),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_same_text_is_one_symbol"),
        word: Word::Succeeds,
        input: || || Ok(interning_twice_gives_one_symbol()),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_symbol_reads_as_its_text"),
        word: Word::Succeeds,
        input: || || Ok(a_symbol_reads_as_its_text()),
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
