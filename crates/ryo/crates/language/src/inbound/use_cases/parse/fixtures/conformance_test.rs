//! The conformance's rendering of rows, on values alone.

use super::super::findings::Findings;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::collections::relations::entry::item::EntryRow;
use crate::domain::relations::constants::NO_SYMBOL;
use crate::domain::relations::service::node_id;
use crate::domain::relations::types::{
    FileId, Flag, Integer, KindBits, Offset, Position, Relation, Rows as _, Span,
};
use crate::domain::symbols::service::intern_derived;
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;
use crate::domain::terms::types::NameText;
use crate::domain::terms::types::ValueKind;
use crate::inbound::use_cases::parse::fixtures::contract::Collect;
use crate::inbound::use_cases::parse::fixtures::expected::{Cell, Fixtures};
use crate::inbound::use_cases::parse::fixtures::expected::{Leafness, Shape, Walked};
use crate::inbound::use_cases::parse::fixtures::rows::{Read, compare, rows_of_relation};

/// A scenario: what a row asks of the rendering.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

/// A read of one file in which `entry` has two rows, the key of the first holding a tab and a line break.
fn two_entries() -> Read {
    let mut symbols = Symbols::default();
    let mut out = Collect::default();
    for (number, key) in ["a\tb\nc", "d"].into_iter().enumerate() {
        let at = u32::try_from(number).unwrap_or(0);
        out.write(EntryRow {
            decoded: intern_derived(&mut symbols, ByteSlice(&[]), NameText(key)),
            key: intern_derived(&mut symbols, ByteSlice(&[]), NameText(key)),
            kind: ValueKind::String,
            occurrence: node_id(FileId(0), Position(at)),
            ordinal: Integer(i64::from(at)),
            owner: node_id(FileId(0), Position(0)),
            file: FileId(0),
            span: Span {
                start: Offset(at),
                end: Offset(at + 1),
            },
            text: intern_derived(&mut symbols, ByteSlice(&[]), NameText(key)),
            boolean: Flag::Lowered,
            integer: Integer(0),
            float: NO_SYMBOL,
            datetime: NO_SYMBOL,
            radix: Integer(0),
        });
    }
    Read {
        out,
        symbols,
        bytes: Vec::new(),
        files: Vec::new(),
        texts: vec![Vec::new()],
        unknowns: &[],
    }
}

/// A name keeps its tab and its line break: a cell is typed, and nothing escapes it.
fn a_name_with_a_tab_and_a_break_is_one_cell() -> usize {
    let rows = rows_of_relation(&two_entries(), Relation::Entry, "entry/input.t");
    usize::from(
        rows.len() == 2
            && rows
                .iter()
                .any(|row| row.contains(&Cell::Name("a\tb\nc".to_owned()))),
    )
}

/// A row written twice is a finding, and so is a row missing from the expected ones.
fn a_repeated_row_and_a_missing_row_are_findings() -> usize {
    let mut findings = Findings(Vec::new());
    let (ab, cd) = (
        vec![Cell::Name("a\tb".to_owned())],
        vec![Cell::Name("c\td".to_owned())],
    );
    let got = vec![ab.clone(), ab.clone()];
    compare(
        "entry",
        &got,
        std::slice::from_ref(&ab),
        &|row| format!("{row:?}"),
        &mut findings,
    );
    compare(
        "entry",
        &got[..1],
        &[ab, cd],
        &|row| format!("{row:?}"),
        &mut findings,
    );
    findings.0.len()
}

/// A language that answers every relation as not in it, reads no file and has no fixtures fails the conformance, the vacuous crate
/// included: the shape of the tree is what every language has, and a language reads something and has samples of it.
fn the_language_that_reads_nothing_is_refused() -> usize {
    let empty = std::env::temp_dir().join("ryo-conformance-no-fixtures");
    let outcome = std::panic::catch_unwind(|| {
        super::conformance(
            "nothing",
            &crate::inbound::use_cases::parse::use_case::ParseUseCaseImpl,
            &crate::inbound::use_cases::project::use_case::ProjectUseCaseImpl,
            &empty,
            &Fixtures {
                kinds: Vec::new(),
                relations: Vec::new(),
                manifests: Vec::new(),
                shape: Shape::Walk(|_| Walked {
                    kind: KindBits(0),
                    leaf: Leafness::Node,
                    start: Offset(0),
                    end: Offset(0),
                    children: Vec::new(),
                }),
                oracle: |_, _| Vec::new(),
                unfilled: Vec::new(),
            },
        );
    });
    usize::from(outcome.is_err())
}

/// An input named for a whole file name is read as that file; one named for an extension, or none, is read as it is: the count of
/// the paths read as expected.
fn an_input_named_for_a_whole_file_name_is_read_as_that_file() -> usize {
    [
        ("entry/input.go.mod", "entry/go.mod"),
        ("input.go.mod", "go.mod"),
        ("entry/input.go", "entry/input.go"),
        ("relations/entry/input.d.ts", "relations/entry/d.ts"),
        ("entry/expected.rs", "entry/expected.rs"),
    ]
    .into_iter()
    .filter(|(path, read)| super::super::findings::read_as(path) == *read)
    .count()
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_name_with_a_tab_and_a_break_is_one_cell"),
        word: Word::Succeeds,
        input: || a_name_with_a_tab_and_a_break_is_one_cell,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_language_that_reads_nothing_is_refused"),
        word: Word::Succeeds,
        input: || the_language_that_reads_nothing_is_refused,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_repeated_row_and_a_missing_row_are_findings"),
        word: Word::Succeeds,
        input: || a_repeated_row_and_a_missing_row_are_findings,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("an_input_named_for_a_whole_file_name_is_read_as_that_file"),
        word: Word::Succeeds,
        input: || an_input_named_for_a_whole_file_name_is_read_as_that_file,
        expect: Expect::Value(5),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
