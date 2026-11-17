//! The parse service read over `Expanding`'s table: the semantic readers the engine shares, with a tree added under the root.

use super::Expanding;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::answers::types::Expansions;
use crate::domain::elements::types::Elements;
use crate::domain::files::types::SourcePath;
use crate::domain::symbols::service::text_of;
use crate::domain::symbols::types::{ByteSlice, Symbols};
use crate::inbound::use_cases::parse::errors::ParseError;
use crate::inbound::use_cases::parse::fixtures::collect::Collect;
use crate::inbound::use_cases::parse::service::read;
use crate::inbound::use_cases::parse::types::{FieldRequest, NoteRequest, ParseRequest};
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{FileId, Position, Relation};
use ryo_relations::domain::terms::types::Symbol;

/// A scenario: what a row asks of the read.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ParseError;

/// The semantic rows of the file `ab` of `Expanding`, read through the parse service with its semantic slots answered by the
/// expanded readers (`Included`) or the native ones (`Ignored`), and the symbols the rows name.
fn semantic_rows(projection: Expansions) -> (Vec<(Relation, Vec<u64>)>, Symbols) {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut out = Collect::default();
    let answered = read(
        &Expanding(projection),
        ParseRequest {
            path: SourcePath("a.small"),
            file: FileId(0),
            bytes: ByteSlice(EXPANDING_SOURCE),
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Unwanted,
        },
        &mut elements,
        &mut symbols,
        &[Relation::SemanticSyntax, Relation::SemanticChild],
        &mut out,
    );
    assert_eq!(answered, Ok(()));
    (out.rows(), symbols)
}

/// The bytes of the file `Expanding` reads.
const EXPANDING_SOURCE: &[u8] = b"ab";

/// The node id of the element at `position` of the file read.
fn id_at(position: u32) -> u64 {
    node_id(FileId(0), Position(position)).0
}

/// The text a row's symbol word names.
fn named(symbols: &Symbols, word: u64) -> Option<&str> {
    let symbol = Symbol(u32::try_from(word).ok()?);
    text_of(symbols, ByteSlice(EXPANDING_SOURCE), symbol).map(|text| text.0)
}

/// A span word's start and end.
fn span_of(word: u64) -> (u64, u64) {
    (word >> 32, word & u64::from(u32::MAX))
}

/// The added element (position 3: kind `added` over `ab`) has its row: id, kind, text, file and span.
fn an_expanded_semantic_syntax_writes_the_added_element_with_its_fields() -> usize {
    let (rows, symbols) = semantic_rows(Expansions::Included);
    rows.iter()
        .filter(|(relation, row)| {
            *relation == Relation::SemanticSyntax
                && row[0] == id_at(3)
                && named(&symbols, row[1]) == Some("added")
                && named(&symbols, row[2]) == Some("ab")
                && row[3] == 0
                && span_of(row[4]) == (0, 2)
        })
        .count()
}

/// The added element's edge goes to its holder, the root, at the ordinal after the root's own two letters.
fn an_expanded_semantic_child_puts_the_added_element_after_its_holders_own_children() -> usize {
    let (rows, _) = semantic_rows(Expansions::Included);
    let own = rows
        .iter()
        .filter(|(relation, row)| {
            *relation == Relation::SemanticChild && row[0] == id_at(0) && row[1] != id_at(3)
        })
        .count();
    rows.iter()
        .filter(|(relation, row)| {
            *relation == Relation::SemanticChild
                && row[0] == id_at(0)
                && row[1] == id_at(3)
                && usize::try_from(row[2]) == Ok(own)
                && own == 2
                && row[4] == 0
        })
        .count()
}

/// The expanded readers write every row the native readers write: the root, both letters and both of their edges.
fn an_expanded_semantic_reader_keeps_every_native_row() -> usize {
    let (expanded, _) = semantic_rows(Expansions::Included);
    let (native, _) = semantic_rows(Expansions::Ignored);
    native.iter().filter(|row| expanded.contains(row)).count()
}

/// The native readers write no row of the added element, neither its own nor its edge.
fn a_native_semantic_reader_writes_no_row_of_an_added_element() -> usize {
    let (rows, _) = semantic_rows(Expansions::Ignored);
    rows.iter()
        .filter(|(relation, row)| {
            (*relation == Relation::SemanticSyntax && row[0] == id_at(3))
                || (*relation == Relation::SemanticChild && row[1] == id_at(3))
        })
        .count()
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_expanded_semantic_syntax_writes_the_added_element_with_its_fields"),
        word: Word::Succeeds,
        input: || an_expanded_semantic_syntax_writes_the_added_element_with_its_fields,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName(
            "an_expanded_semantic_child_puts_the_added_element_after_its_holders_own_children",
        ),
        word: Word::Succeeds,
        input: || an_expanded_semantic_child_puts_the_added_element_after_its_holders_own_children,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_expanded_semantic_reader_keeps_every_native_row"),
        word: Word::Succeeds,
        input: || an_expanded_semantic_reader_keeps_every_native_row,
        expect: Expect::Value(5),
    },
    Case {
        name: CaseName("a_native_semantic_reader_writes_no_row_of_an_added_element"),
        word: Word::Succeeds,
        input: || a_native_semantic_reader_writes_no_row_of_an_added_element,
        expect: Expect::Value(0),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
