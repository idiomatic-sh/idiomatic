//! The `found_rows` service: free functions named by verb that render the rows the new engine's scan kept of one file as the
//! cells of the answer, in the layout every row of the answer has, and compare one file's rows as the two engines rendered them;
//! with no IO and no ports.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::types::{CellCompared, Compared, ComparedRow, FoundInput, RowDifference};
use crate::domain::finish::errors::FinishError;
use crate::domain::finish::service::{fill_line_starts, place_in};
use crate::domain::finish::types::{ColumnNumber, FileLine, LineStarts, Point};
use crate::domain::render::service::{lent_row, placed_row, row_cells, shown_cells};
use crate::domain::render::types::{AnswerCell, ColumnPlace, Rendered, Shown, ShownRow};
use crate::domain::value_cells::service::value_cell;
use ryo_relations::domain::relations::service::from_bits;
use ryo_relations::domain::relations::types::{Bits, CellKind};
use ryo_values::domain::plan_parts::types::{ColumnIndex, RowWidth};
use ryo_values::domain::records::service::keep;
use ryo_values::domain::row_texts::types::{Cell, CellText, RowsTextView, TextRange};
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::runs::types::RuleNumber;
use ryo_values::domain::scan_rows::types::{KeptCell, RowPlace, ScanRow};

/// Renders the first `limit` of `rows`, the rows one rule found in the scene's file, in the order they are handed, onto `into`,
/// as the cells the layout asks for, each row with its place and the rule that found it, and counts every row: a row past the
/// limit is never made into cells. A word's cell is its kept text; every other cell is the value its column's kind says its
/// number is. The room `into` and `starts` have is reused.
///
/// # Errors
///
/// `NoSuchColumn` when a row lacks the subject column or a shown column.
pub(crate) fn render_found(
    input: FoundInput<'_>,
    rows: &[ScanRow],
    starts: &mut LineStarts,
    into: &mut Rendered,
) -> Result<(), FinishError> {
    let FoundInput {
        rule,
        emit,
        kinds,
        layout,
        scene,
        limit,
    } = input;
    into.total.0 += u64::try_from(rows.len()).unwrap_or(u64::MAX);
    let shown = rows.get(..limit.0.min(rows.len())).unwrap_or(rows);
    let starting = shown.iter().filter_map(|row| match row.place {
        RowPlace::At(span) => Some(span.start),
        RowPlace::Unplaced => None,
    });
    if let Some(last) = starting.max() {
        fill_line_starts(scene.source, last, starts);
    }
    let unplaced = Point {
        line: FileLine(0),
        column: ColumnNumber(0),
    };
    for row in shown {
        // A row whose subject column is missing is refused, whatever it shows.
        if row.cells.get(usize::from(emit.subject)).is_none() {
            return Err(RowsError::NoSuchColumn {
                index: emit.subject.0,
            }
            .into());
        }
        let place = match row.place {
            RowPlace::At(span) => place_in(scene.source, starts, span.start),
            RowPlace::Unplaced => unplaced,
        };
        let shape = ShownRow {
            shown: layout.shown,
            emit,
            width: RowWidth(row.cells.len()),
        };
        shown_cells(into, shape, &mut |text, index| {
            let at = usize::from(index);
            match (row.cells.get(at), kinds.get(at)) {
                (Some(KeptCell::Text(word)), _) => {
                    Ok(Some(keep(text, Cell::Text(CellText(&word.0)))))
                }
                (Some(KeptCell::Number(bits)), Some(kind)) => {
                    Ok(value_cell(text, from_bits(*kind, Bits(bits.0)), scene))
                }
                (Some(KeptCell::Number(_)), None) | (None, _) => {
                    Err(RowsError::NoSuchColumn { index: index.0 })
                }
            }
        })?;
        placed_row(into, rule, place);
    }
    Ok(())
}

/// Whether each cell of a rule's rendered row is compared, in the order the row's layout puts them: a cell that holds a node
/// is not. The line and the character that end every row always are.
fn compared_cells(compared: &Compared<'_>) -> Vec<CellCompared> {
    let of = |place: ColumnPlace| match compared.kinds.get(place.0) {
        Some(CellKind::Node) => CellCompared::Node,
        Some(
            CellKind::Symbol | CellKind::File | CellKind::Span | CellKind::Number | CellKind::Flag,
        )
        | None => CellCompared::Compared,
    };
    let column = |index: &ColumnIndex| of(ColumnPlace(usize::from(*index)));
    let mut cells = Vec::new();
    match compared.shown {
        Shown::All => cells.extend((0..compared.kinds.len()).map(ColumnPlace).map(of)),
        Shown::Placed(placed) => {
            cells.push(CellCompared::Compared);
            cells.extend(placed.cells.iter().map(|each| match each {
                AnswerCell::Column(own) => of(*own),
                AnswerCell::Empty => CellCompared::Compared,
            }));
        }
        Shown::Finding(_) => {
            let emit = compared.emit;
            cells.extend([
                CellCompared::Compared,
                column(&emit.subject),
                CellCompared::Compared,
            ]);
            cells.extend(emit.key.iter().map(column));
            let columns = emit.headings.len().min(compared.kinds.len());
            cells.extend(
                (0..columns)
                    .map(ColumnPlace)
                    .flat_map(|place| [CellCompared::Compared, of(place)]),
            );
        }
    }
    cells
}

/// A rendered row as the text it is compared by: its cells joined by tabs; a node's cell is `#`, a cell with no value nothing.
fn compared_row(
    text: RowsTextView<'_>,
    cells: &[Option<Cell<TextRange>>],
    compared: &[CellCompared],
) -> ComparedRow {
    let mut written = String::new();
    for (place, cell) in lent_row(text, cells).enumerate() {
        if place > 0 {
            written.push('\t');
        }
        // Writing to a string never fails.
        match cell {
            Some(_) if compared.get(place) == Some(&CellCompared::Node) => written.push('#'),
            Some(Cell::File(path) | Cell::Text(path)) => written.push_str(path.0),
            Some(Cell::Number(number)) => write!(written, "{}", number.0).unwrap_or_default(),
            Some(Cell::Span(span)) => {
                write!(written, "{}..{}", span.start.0, span.end.0).unwrap_or_default();
            }
            None => {}
        }
    }
    ComparedRow(written.into())
}

/// The rows of the compared rules in one rendering of a file, rule by rule, each as the text it is compared by, sorted.
fn compared_rows<'a>(
    rendered: &Rendered,
    compared: &dyn Fn(RuleNumber) -> Option<Compared<'a>>,
) -> BTreeMap<RuleNumber, Vec<ComparedRow>> {
    let text = RowsTextView(&rendered.text.0);
    let mut rows: BTreeMap<RuleNumber, Vec<ComparedRow>> = BTreeMap::new();
    let mut held: BTreeMap<RuleNumber, Vec<CellCompared>> = BTreeMap::new();
    for each in &rendered.order {
        let Some(how) = compared(each.rule) else {
            continue;
        };
        let compared = held
            .entry(each.rule)
            .or_insert_with(|| compared_cells(&how));
        let cells = row_cells(&rendered.cells, &rendered.ends, each.row);
        rows.entry(each.rule)
            .or_default()
            .push(compared_row(text, cells, compared));
    }
    for each in rows.values_mut() {
        each.sort_unstable();
    }
    rows
}

/// The first row at which the two engines' renderings of one file differ, for the rules `compared` says how to compare: none
/// when every such rule has the same rows in both, whatever order they were rendered in. A cell that holds a node is not
/// compared.
#[must_use]
pub(crate) fn first_difference<'a>(
    old: &Rendered,
    new: &Rendered,
    compared: &dyn Fn(RuleNumber) -> Option<Compared<'a>>,
) -> Option<RowDifference> {
    let (before, after) = (compared_rows(old, compared), compared_rows(new, compared));
    let nothing = Vec::new();
    let rules: std::collections::BTreeSet<RuleNumber> =
        before.keys().chain(after.keys()).copied().collect();
    rules.into_iter().find_map(|rule| {
        let (old, new) = (
            before.get(&rule).unwrap_or(&nothing),
            after.get(&rule).unwrap_or(&nothing),
        );
        let at = (0..old.len().max(new.len())).find(|at| old.get(*at) != new.get(*at))?;
        Some(RowDifference {
            rule,
            old: old.get(at).cloned(),
            new: new.get(at).cloned(),
        })
    })
}
