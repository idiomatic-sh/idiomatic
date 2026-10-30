//! The `choosing` service's cases, on values alone.

use super::service::chosen;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::finish::errors::FinishError;
use crate::domain::finish::types::{ColumnNumber, FileLine, RuleNumber};
use crate::domain::render::service::{end_row, key_count, lent_row};
use crate::domain::render::types::{KeyCount, Rendered, RowNumber, RowOrder};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::records::service::keep;
use ryo_values::domain::records::types::Cell as Typed;
use ryo_values::domain::records::types::RowTotal;
use ryo_values::domain::records::types::{CellText, RowsTextView};

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Chooses among three rows (of the paths `b.rs`, `a.rs` and `b.rs`) the rows of the files asked for; none asked for is every row.
    Chosen(&'static [&'static str]),
}

/// What a row of the cases does.
type Input = Scenario;
/// Which of the three rows were chosen, as the bits 1, 2 and 4.
type Value = usize;
type Failure = FinishError;

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

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::Chosen(only) => {
            let mut here = Rendered {
                total: RowTotal(3),
                ..Rendered::default()
            };
            for (row, (rule, path)) in [("0", "b.rs"), ("1", "a.rs"), ("2", "b.rs")]
                .into_iter()
                .enumerate()
            {
                finding_row(&mut here, rule, path, "k");
                here.order.push(RowOrder {
                    rule: RuleNumber(0),
                    line: FileLine(0),
                    column: ColumnNumber(0),
                    row: RowNumber(u32::try_from(row).unwrap_or(0)),
                });
            }
            let only: Vec<FilePath> = only.iter().map(|path| FilePath((*path).into())).collect();
            // The rows chosen are told by their rule's id, the place of the row.
            let text = RowsTextView(&here.text);
            chosen(&here, &only)
                .map(|cells| match lent_row(text, cells).next() {
                    Some(Some(Typed::Text(CellText("0")))) => 1,
                    Some(Some(Typed::Text(CellText("1")))) => 2,
                    Some(Some(Typed::Text(CellText("2")))) => 4,
                    _ => 0,
                })
                .sum()
        }
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("with_no_file_asked_for_every_row_is_chosen"),
        word: Word::Succeeds,
        input: || Scenario::Chosen(&[]),
        expect: Expect::Value(7),
    },
    Case {
        name: CaseName("only_the_rows_of_the_files_asked_for_are_chosen"),
        word: Word::Succeeds,
        input: || Scenario::Chosen(&["b.rs"]),
        expect: Expect::Value(5),
    },
    Case {
        name: CaseName("a_file_asked_for_that_has_no_rows_chooses_none"),
        word: Word::Succeeds,
        input: || Scenario::Chosen(&["c.rs"]),
        expect: Expect::Value(0),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<_, Failure>(subject(input))
    });
}
