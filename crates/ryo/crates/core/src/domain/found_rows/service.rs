//! The `found_rows` service: free functions named by verb that render the rows the new engine's scan kept of one file as the
//! cells of the answer, in the layout every row of the answer has; with no IO and no ports.

use super::types::FoundInput;
use crate::domain::finish::errors::FinishError;
use crate::domain::finish::service::{fill_line_starts, place_in};
use crate::domain::finish::types::{ColumnNumber, FileLine, LineStarts, Point};
use crate::domain::render::service::{placed_row, shown_cells};
use crate::domain::render::types::{Rendered, ShownRow};
use crate::domain::value_cells::service::value_cell;
use ryo_relations::domain::relations::service::from_bits;
use ryo_relations::domain::relations::types::Bits;
use ryo_values::domain::plan_parts::types::RowWidth;
use ryo_values::domain::records::service::keep;
use ryo_values::domain::row_texts::types::{Cell, CellText};
use ryo_values::domain::rows::errors::RowsError;
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
