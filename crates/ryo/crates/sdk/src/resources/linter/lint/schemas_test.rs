//! What the schemas of this entry accept and refuse, one row of `CASES` for each input.

use super::report;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::cell::types::{Cell as Typed, HostText, Slot};
use crate::domain::file::types::RelativePath;
use crate::resources::linter::lint::types::Glob;
use crate::resources::linter::lint::types::Place;
use crate::resources::linter::lint::types::RuleId;
use ryo_core::inbound::use_cases::lint::types::LintReport as CoreReport;
use ryo_core::outbound::rows::factory::collect;
use ryo_core::outbound::rows::interface::RowsPort as _;
use ryo_core::outbound::rows::types::LentCell;
use ryo_language::domain::relations::types::Integer;
use ryo_values::domain::records::types::{Cell, CellText};

/// A text and what it is offered as.
#[derive(Clone, Copy, Debug)]
enum Offered {
    Id(&'static str),
    Glob(&'static str),
    /// The typed cells of a finding the run wrote: rule, subject, how many key values and each, each column as its heading and its
    /// value, line and character.
    Row(&'static [Option<Cell<CellText<'static>>>]),
    /// The same, read for its columns: the count of columns when each reads whole, its name and its value typed as the SDK hands it.
    Columns(&'static [Option<Cell<CellText<'static>>>]),
}

/// What a row gives the schemas: a text and what it is offered as.
type Input = Offered;
/// How long the text the type holds is; for a finding, its line times a thousand plus its character, or zero with no place.
type Value = usize;
/// Why the schemas refuse.
type Failure = Refused;

/// Which conversion refused.
#[derive(Debug, PartialEq, Eq)]
enum Refused {
    Id,
    Glob,
    Row,
    Columns,
}

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_id_with_text_is_accepted"),
        word: Word::Succeeds,
        input: || Offered::Id("no-env-reads"),
        expect: Expect::Value(12),
    },
    Case {
        name: CaseName("a_blank_rule_id_is_refused"),
        word: Word::Refuses,
        input: || Offered::Id("  "),
        expect: Expect::Error(Refused::Id),
    },
    Case {
        name: CaseName("a_glob_that_parses_is_accepted"),
        word: Word::Succeeds,
        input: || Offered::Glob("src/**/*.rs"),
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName("a_glob_that_does_not_parse_is_refused"),
        word: Word::Refuses,
        input: || Offered::Glob("["),
        expect: Expect::Error(Refused::Glob),
    },
    Case {
        name: CaseName("a_finding_carries_its_line_and_character"),
        word: Word::Succeeds,
        input: || {
            Offered::Row(&[
                Some(Cell::Text(CellText("r"))),
                Some(Cell::File(CellText("a.rs"))),
                Some(Cell::Number(Integer(1))),
                Some(Cell::Text(CellText("k"))),
                Some(Cell::Number(Integer(2))),
                Some(Cell::Number(Integer(4))),
            ])
        },
        expect: Expect::Value(2004),
    },
    Case {
        name: CaseName("a_finding_with_no_place_has_none"),
        word: Word::Succeeds,
        input: || {
            Offered::Row(&[
                Some(Cell::Text(CellText("r"))),
                Some(Cell::File(CellText("a.rs"))),
                Some(Cell::Number(Integer(1))),
                Some(Cell::Text(CellText("k"))),
                None,
                None,
            ])
        },
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_finding_carries_every_column_of_its_row_whole"),
        word: Word::Succeeds,
        input: || {
            Offered::Columns(&[
                Some(Cell::Text(CellText("r"))),
                Some(Cell::File(CellText("src/a.rs"))),
                Some(Cell::Number(Integer(0))),
                Some(Cell::Text(CellText("path"))),
                Some(Cell::File(CellText("src/a.rs"))),
                Some(Cell::Text(CellText("span"))),
                Some(Cell::Text(CellText("1:2,3:4"))),
                None,
                None,
            ])
        },
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_column_with_no_heading_is_refused"),
        word: Word::Refuses,
        input: || {
            Offered::Columns(&[
                Some(Cell::Text(CellText("r"))),
                Some(Cell::File(CellText("src/a.rs"))),
                Some(Cell::Number(Integer(0))),
                Some(Cell::Text(CellText("path"))),
                Some(Cell::File(CellText("src/a.rs"))),
                Some(Cell::Number(Integer(9))),
                None,
                None,
            ])
        },
        expect: Expect::Error(Refused::Columns),
    },
    Case {
        name: CaseName("a_finding_whose_line_is_no_number_is_refused"),
        word: Word::Refuses,
        input: || {
            Offered::Row(&[
                Some(Cell::Text(CellText("r"))),
                Some(Cell::File(CellText("a.rs"))),
                Some(Cell::Number(Integer(1))),
                Some(Cell::Text(CellText("k"))),
                Some(Cell::Text(CellText("two"))),
                Some(Cell::Number(Integer(4))),
            ])
        },
        expect: Expect::Error(Refused::Row),
    },
];

/// Converts one input through the schemas and answers how long the text the type holds is.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Offered::Id(text) => RuleId::try_from(text)
            .map(|id| id.to_string().len())
            .map_err(|_refused| Refused::Id),
        Offered::Glob(text) => Glob::try_from(text)
            .map(|glob| glob.to_string().len())
            .map_err(|_refused| Refused::Glob),
        Offered::Columns(cells) => {
            let read = written(cells).ok_or(Refused::Columns)?.columns;
            let whole = read.first().is_some_and(|column| {
                &*column.name.0 == "path"
                    && column.value == Slot::Is(Typed::File(RelativePath("src/a.rs".into())))
            }) && read.get(1).is_none_or(|column| {
                &*column.name.0 == "span"
                    && column.value == Slot::Is(Typed::Text(HostText("1:2,3:4".into())))
            });
            Ok(if whole { read.len() } else { 0 })
        }
        Offered::Row(cells) => {
            let finding = written(cells).ok_or(Refused::Row)?;
            match finding.place {
                Place::At { line, column } => {
                    Ok(usize::try_from(line.0 * 1000 + column.0).unwrap_or(0))
                }
                Place::Missing => Ok(0),
            }
        }
    }
}

/// The one finding the report reads of a run that wrote one row of `cells`; none when the report refuses the row.
fn written(
    cells: &[Option<Cell<CellText<'_>>>],
) -> Option<crate::resources::linter::lint::types::Finding> {
    let collected = ryo_core::outbound::rows::types::Collector::default();
    let collector = collect(collected.clone());
    collector.row(&mut cells.iter().copied().map(LentCell::from));
    match report(collected, CoreReport::default()) {
        Ok(read) => read.findings.into_iter().next(),
        Err(_refused) => None,
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
