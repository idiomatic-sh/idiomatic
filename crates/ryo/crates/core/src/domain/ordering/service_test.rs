//! The `ordering` service's cases, on values alone.

use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::relations::types::{CellKind, FileId, Offset, Span, Value as Cell};
use ryo_language::domain::symbols::service::intern;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;

use super::service::{append, offer, sort_findings};
use super::types::{Ordered, Turn};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::finish::errors::FinishError;
use crate::domain::finish::types::{ColumnNumber, FileLine, LineStarts, RuleNumber};
use crate::domain::render::service::{end_row, key_count, lent_row, render_rows, row_cells};
use crate::domain::render::types::{
    FindingRule, KeyCount, RenderInput, Rendered, RowLayout, RowLimit, RowNumber, RowOrder, Shown,
};
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::Table;
use crate::domain::value_cells::types::Scene;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::records::service::keep;
use ryo_values::domain::records::types::Cell as Typed;
use ryo_values::domain::records::types::RowTotal;
use ryo_values::domain::records::types::{CellText, RowsTextView};

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Offers one file's turn twice.
    OfferedTwice,
    /// Renders findings of two keys and puts them in order.
    Sorted,
    /// Renders the same findings twice and puts them in order: a finding is its rule, path and key, so each is one.
    Repeated,
    /// Offers files out of order and reads the order they leave in.
    Ordered,
    /// Offers three files of two rows each with a limit of three: three leave, six are counted.
    Counted,
    /// The findings of two rules rendered apart go out as if they were rendered together.
    Appended,
}

/// What a row of the cases does.
type Input = Scenario;
/// Whether the check held.
type Value = usize;
type Failure = FinishError;

const SOURCE: &[u8] = b"fn a() {}\nfn bee() {}\n";

/// A finding's cells: its rule, subject, key and columns, then its line and character.
const FINDING: RowLayout<'static> = RowLayout {
    shown: Shown::Finding(FindingRule("relation")),
};

fn emit() -> Emit {
    Emit {
        step: StepId(0),
        subject: ColumnIndex(0),
        key: vec![ColumnIndex(2)],
        headings: Vec::new(),
    }
}

fn rows(symbols: &mut Symbols) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::File, CellKind::Symbol, CellKind::Span],
        ..Table::default()
    };
    for (start, end) in [(13, 16), (3, 4)] {
        let name = intern(symbols, ByteSlice(SOURCE), Offset(start), Offset(end));
        let span = Span {
            start: Offset(start),
            end: Offset(end),
        };
        push_row(
            &mut table,
            &[Cell::File(FileId(0)), Cell::Symbol(name), Cell::Span(span)],
        );
    }
    table
}

/// The cells of the row rendered `row`th, lent.
fn lent_cells(rendered: &Rendered, row: RowNumber) -> Vec<Option<Typed<CellText<'_>>>> {
    lent_row(
        RowsTextView(&rendered.text),
        row_cells(&rendered.cells, &rendered.ends, row),
    )
    .collect()
}

/// Adds a finding's row by hand: its rule's id, its path, one key and no column, with no place.
fn finding_row(into: &mut Rendered, rule: &str, path: &str, key: &str) {
    let id = keep(&mut into.text, Typed::Text(CellText(rule)));
    into.cells.push(Some(id));
    let file = keep(&mut into.text, Typed::File(CellText(path)));
    into.cells.push(Some(file));
    into.cells.push(Some(key_count(KeyCount(1))));
    let named = keep(&mut into.text, Typed::Text(CellText(key)));
    into.cells.push(Some(named));
    into.cells.extend([None, None]);
    end_row(into);
}

/// Every row offered is counted, and only as many leave as the limit allows, to the files in the order they are offered.
fn offered_rows_are_counted() -> Value {
    let mut ordered = Ordered {
        limit: RowTotal(3),
        ..Ordered::default()
    };
    let mut leaving = Vec::new();
    for file in [2_usize, 0, 1] {
        let mut here = Rendered {
            total: RowTotal(2),
            ..Rendered::default()
        };
        for row in 0..2 {
            here.cells.push(Some(Typed::Number(Integer(1))));
            end_row(&mut here);
            here.order.push(RowOrder {
                rule: RuleNumber(0),
                line: FileLine(0),
                column: ColumnNumber(0),
                row: RowNumber(row),
            });
        }
        let mut ready = offer(&mut ordered, Turn(file), here);
        while let Some(each) = ready {
            leaving.push(super::service::allowance(&mut ordered, &each).0);
            super::service::recycle(&mut ordered, each);
            ready = super::service::next_ready(&mut ordered);
        }
    }
    usize::from(
        ordered.total == RowTotal(6) && ordered.shown == RowTotal(3) && leaving == [2, 1, 0],
    )
}

/// The findings of two rules appended together sort by their file, not by their rule.
fn appended_findings_sort() -> Value {
    let rendered_by = |rule: u32, path: &str, key: &str| {
        let mut here = Rendered {
            total: RowTotal(1),
            ..Rendered::default()
        };
        finding_row(&mut here, &rule.to_string(), path, key);
        here.order.push(RowOrder {
            rule: RuleNumber(rule),
            line: FileLine(0),
            column: ColumnNumber(0),
            row: RowNumber(0),
        });
        here
    };
    let mut together = Rendered::default();
    append(&mut together, &rendered_by(0, "b.rs", "x"));
    append(&mut together, &rendered_by(1, "a.rs", "y"));
    sort_findings(&mut together);
    let first = together.order[0].row;
    usize::from(
        together.order.len() == 2
            && together.total == RowTotal(2)
            && lent_cells(&together, first)
                == [
                    Some(Typed::Text(CellText("1"))),
                    Some(Typed::File(CellText("a.rs"))),
                    Some(Typed::Number(Integer(1))),
                    Some(Typed::Text(CellText("y"))),
                    None,
                    None,
                ]
            && lent_cells(&together, together.order[1].row).get(1)
                == Some(&Some(Typed::File(CellText("b.rs")))),
    )
}

/// The files leave in the order of their turns, whatever the order they are offered in.
fn files_leave_in_their_turn() -> Value {
    let mut ordered = Ordered::default();
    let mut written = Vec::new();
    for file in [1_usize, 2, 0] {
        let mut here = Rendered::default();
        here.cells.push(Some(Typed::Number(Integer(
            i64::try_from(file).unwrap_or(0),
        ))));
        end_row(&mut here);
        let mut ready = offer(&mut ordered, Turn(file), here);
        while let Some(each) = ready {
            super::service::account(&mut ordered, &each);
            written.push(each.cells.clone());
            super::service::recycle(&mut ordered, each);
            ready = super::service::next_ready(&mut ordered);
        }
    }
    let number = |each: i64| vec![Some(Typed::Number(Integer(each)))];
    usize::from(written == [number(0), number(1), number(2)])
}

fn offered_twice() -> Value {
    let message = crash_of(|| {
        let mut ordered = Ordered::default();
        for _ in 0_u8..2 {
            let mut here = Rendered::default();
            here.cells.push(Some(Typed::Number(Integer(1))));
            end_row(&mut here);
            assert!(offer(&mut ordered, Turn(1), here).is_none());
        }
    });
    usize::from(message == "a turn offered once")
}

/// What `work` crashed with: its panic's message, or nothing when it did not crash.
fn crash_of(work: impl FnOnce()) -> String {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).map_or_else(
        |payload| {
            payload
                .downcast_ref::<&str>()
                .map(|message| (*message).to_owned())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default()
        },
        |()| String::new(),
    )
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let paths = vec![FilePath("lib.rs".into())];
    let mut symbols = Symbols::default();
    let table = rows(&mut symbols);
    let elements = Elements::default();
    let scene = Scene {
        unknown: &[],
        file: FileId(0),
        symbols: &symbols,
        elements: &elements,
        source: SourceView(SOURCE),
        paths: &paths,
    };
    let mut starts = LineStarts(Vec::new());
    let mut rendered = Rendered::default();
    match scenario {
        Scenario::OfferedTwice => Ok(offered_twice()),
        Scenario::Sorted => {
            render_rows(
                RenderInput {
                    rule: RuleNumber(0),
                    table: &table,
                    emit: &emit(),
                    layout: FINDING,
                    scene,
                    limit: RowLimit(usize::MAX),
                },
                &mut starts,
                &mut rendered,
            )?;
            sort_findings(&mut rendered);
            // The key is a span: spans sort as numbers, so `3..4` comes before `13..16`.
            Ok(usize::from(
                lent_cells(&rendered, rendered.order[0].row)
                    == [
                        Some(Typed::Text(CellText("relation"))),
                        Some(Typed::File(CellText("lib.rs"))),
                        Some(Typed::Number(Integer(1))),
                        Some(Typed::Span(Span {
                            start: Offset(3),
                            end: Offset(4),
                        })),
                        Some(Typed::Number(Integer(1))),
                        Some(Typed::Number(Integer(4))),
                    ],
            ))
        }
        Scenario::Repeated => {
            render_rows(
                RenderInput {
                    rule: RuleNumber(0),
                    table: &table,
                    emit: &emit(),
                    layout: FINDING,
                    scene,
                    limit: RowLimit(usize::MAX),
                },
                &mut starts,
                &mut rendered,
            )?;
            let once = rendered.order.len();
            render_rows(
                RenderInput {
                    rule: RuleNumber(0),
                    table: &table,
                    emit: &emit(),
                    layout: FINDING,
                    scene,
                    limit: RowLimit(usize::MAX),
                },
                &mut starts,
                &mut rendered,
            )?;
            sort_findings(&mut rendered);
            Ok(usize::from(once > 0 && rendered.order.len() == once))
        }
        Scenario::Counted => Ok(offered_rows_are_counted()),
        Scenario::Appended => Ok(appended_findings_sort()),
        Scenario::Ordered => Ok(files_leave_in_their_turn()),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("findings_go_out_by_key"),
        word: Word::Succeeds,
        input: || Scenario::Sorted,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("findings_with_the_same_rule_path_and_key_are_one"),
        word: Word::Succeeds,
        input: || Scenario::Repeated,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("files_leave_up_to_the_limit_and_every_row_is_counted"),
        word: Word::Succeeds,
        input: || Scenario::Counted,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("findings_rendered_apart_go_out_as_if_rendered_together"),
        word: Word::Succeeds,
        input: || Scenario::Appended,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("files_go_out_in_order_whenever_they_finish"),
        word: Word::Succeeds,
        input: || Scenario::Ordered,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_turn_offered_twice_crashes_instead_of_counting_its_text_twice"),
        word: Word::Succeeds,
        input: || Scenario::OfferedTwice,
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
