//! The `render` service: free functions named by verb that turn the rows of one file into typed cells and
//! let the files leave in order, with no IO and no ports. A name's text is kept here, one row at a
//! time, as the row is made; no cell becomes text for a reader in the core.

use ryo_language::domain::elements::service::start_at;
use ryo_language::domain::names::service::places_of;
use ryo_language::domain::symbols::service::place_of;
use ryo_relations::domain::relations::service::{node_file, node_position};
use ryo_relations::domain::relations::types::{Integer, Name, NodeId};

use super::constants::rule_heading;
use super::types::{
    AnswerCell, AnswerColumns, Answering, ColumnPlace, FindingParts, FindingRule, KeyCount,
    KeyOffset, RenderInput, Rendered, RowKey, RowLayout, RowLimit, RowNumber, RowOrder, RowsAnswer,
    RuleRow, Shown, ShownRow, TableRow,
};
use crate::domain::finish::errors::FinishError;
use crate::domain::finish::service::{
    column_cell, fill_line_starts, line_cell, offset_of, place_in,
};
use {
    crate::domain::finish::types::ColumnNumber, crate::domain::finish::types::FileLine,
    crate::domain::finish::types::LineStarts, crate::domain::finish::types::Point,
    ryo_values::domain::runs::types::RuleNumber,
};

use crate::domain::value_cells::service::value_cell;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Heading, RowWidth};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::service::{keep, lent};
use ryo_values::domain::row_texts::types::RenderedText;
use ryo_values::domain::row_texts::types::{
    Cell, CellText, RowEnds, RowTotal, RowsTextView, TextRange,
};
use ryo_values::domain::rows::errors::RowsError;
use {
    ryo_values::domain::rows::service::cell, ryo_values::domain::rows::service::positions,
    ryo_values::domain::rows::service::row_in,
};

/// The answer of the rules' own columns over `plans`, at most `limit` rows of it written. One rule's columns are the answer's; with
/// more than one, the answer's are the rule's id, then each heading of any rule, as many times as one rule has it, in the order the
/// rules first name it: a rule's row fills its own and leaves the others empty, so no column of any rule is lost.
#[must_use]
pub(crate) fn rows_answer(plans: &[&Plan], limit: RowTotal) -> RowsAnswer {
    if let [plan] = plans {
        return RowsAnswer {
            limit,
            headings: plan.emit.headings.clone(),
            columns: AnswerColumns::One,
        };
    }
    let mut headings: Vec<Heading> = Vec::new();
    let mut cells = Vec::with_capacity(plans.len());
    for plan in plans {
        // The answer's column each of the rule's fills: the first of its heading the rule has not filled yet, or a new one.
        let mut filled: Vec<usize> = Vec::with_capacity(plan.emit.headings.len());
        for heading in &plan.emit.headings {
            let free = headings
                .iter()
                .enumerate()
                .find(|(at, each)| *each == heading && !filled.contains(at))
                .map(|(at, _)| at);
            filled.push(free.unwrap_or_else(|| {
                headings.push(heading.clone());
                headings.len() - 1
            }));
        }
        cells.push(filled);
    }
    let columns = cells
        .into_iter()
        .map(|filled| {
            (0..headings.len())
                .map(|at| {
                    filled
                        .iter()
                        .position(|each| *each == at)
                        .map_or(AnswerCell::Empty, |own| {
                            AnswerCell::Column(ColumnPlace(own))
                        })
                })
                .collect()
        })
        .collect();
    let mut named = vec![Heading(rule_heading().into())];
    named.extend(headings);
    RowsAnswer {
        limit,
        headings: named,
        columns: AnswerColumns::Many(columns),
    }
}

/// What the rows of the rule at `rule`, named `id`, show in `answering`: a finding's cells, or the rule's own.
#[must_use]
pub(crate) fn shown_by<'a>(
    answering: &'a Answering,
    rule: RuleNumber,
    id: FindingRule<'a>,
) -> RowLayout<'a> {
    let shown = match answering {
        Answering::Findings => Shown::Finding(id),
        Answering::Rows(answer) => match &answer.columns {
            AnswerColumns::One => Shown::All,
            AnswerColumns::Many(cells) => Shown::Placed(RuleRow {
                rule: id,
                cells: cells
                    .get(usize::try_from(rule.0).unwrap_or(usize::MAX))
                    .map_or(&[][..], Vec::as_slice),
            }),
        },
    };
    RowLayout { shown }
}

/// How many of a file's rows are made into cells in `answering`: every finding; the rows the answer writes at most.
#[must_use]
pub(crate) fn rendered_limit(answering: &Answering) -> RowLimit {
    match answering {
        Answering::Findings => RowLimit(usize::MAX),
        Answering::Rows(answer) => RowLimit(usize::try_from(answer.limit.0).unwrap_or(usize::MAX)),
    }
}

/// Ends the row being made: it ends where the cells end now.
pub(crate) fn end_row(into: &mut Rendered) {
    into.ends
        .0
        .push(u32::try_from(into.cells.len()).unwrap_or(u32::MAX));
}

/// The next row's number among the rows of `into`.
#[must_use]
pub(crate) fn next_row(into: &Rendered) -> RowNumber {
    RowNumber(u32::try_from(into.ends.0.len()).unwrap_or(u32::MAX))
}

/// The cell of a finding's row that says how many key columns follow it.
#[must_use]
pub(crate) fn key_count<T>(count: KeyCount) -> Cell<T> {
    Cell::Number(Integer(i64::try_from(count.0).unwrap_or(i64::MAX)))
}

/// Renders the first `limit` rows of `table` by place onto `into`, as the cells `layout` asks for,
/// each row with its place and the `rule` that gave it, and counts every row: a row past the
/// limit is never made into cells. The room `into` and `starts` have is reused.
///
/// # Errors
///
/// `NoSuchColumn` when the rows lack the subject column or a shown column.
pub(crate) fn render_rows(
    input: RenderInput<'_>,
    starts: &mut LineStarts,
    into: &mut Rendered,
) -> Result<(), FinishError> {
    let RenderInput {
        rule,
        table,
        emit,
        layout,
        scene,
        limit,
    } = input;
    into.total.0 += u64::try_from(table.count.0).unwrap_or(u64::MAX);
    // A node of this file is placed where its element starts; any other has no place here.
    let nodes = |node: NodeId| {
        let position = node_position(node);
        let at = usize::try_from(position.0).unwrap_or(usize::MAX);
        let implied = (at < scene.elements.start.len())
            .then(|| scene.elements.flags.get(at))
            .flatten()
            .map(|flags| flags.0 & ryo_relations::domain::relations::constants::IMPLIED.0 != 0);
        (node_file(node) == scene.file && implied == Some(false))
            .then(|| start_at(scene.elements, position))
    };
    let names = |name: Name| place_of(name, places_of(scene.symbols), scene.symbols);
    into.keys.clear();
    for index in positions(table.count) {
        if let Some(row) = row_in(table, index) {
            into.keys.push(RowKey {
                offset: offset_of(row, emit, &nodes, &names)
                    .map_or(KeyOffset::Unplaced, KeyOffset::At),
                index: TableRow(u32::try_from(index.0).unwrap_or(u32::MAX)),
            });
        }
    }
    if into.keys.len() > limit.0 {
        if limit.0 > 0 {
            let _: (&mut [RowKey], &mut RowKey, &mut [RowKey]) =
                into.keys.select_nth_unstable(limit.0 - 1);
        }
        into.keys.truncate(limit.0);
    }
    into.keys.sort_unstable();
    if let Some(last) = into
        .keys
        .iter()
        .filter_map(|key| match key.offset {
            KeyOffset::At(offset) => Some(offset),
            KeyOffset::Unplaced => None,
        })
        .max()
    {
        fill_line_starts(scene.source, last, starts);
    }
    let unplaced = Point {
        line: FileLine(0),
        column: ColumnNumber(0),
    };
    let kept = std::mem::take(&mut into.keys);
    let rows = kept.iter().filter_map(|key| {
        row_in(
            table,
            RowIndex(usize::try_from(key.index.0).unwrap_or(usize::MAX)),
        )
    });
    for row in rows {
        // A row whose subject column is missing is refused, whatever it shows.
        if let Err(error) = cell(row, emit.subject) {
            return Err(error.into());
        }
        let place = offset_of(row, emit, &nodes, &names)
            .map_or(unplaced, |offset| place_in(scene.source, starts, offset));
        let shape = ShownRow {
            shown: layout.shown,
            emit,
            width: RowWidth(row.len()),
        };
        shown_cells(into, shape, &mut |text, index| {
            cell(row, index).map(|value| value_cell(text, value, scene))
        })?;
        placed_row(into, rule, place);
    }
    into.keys = kept;
    Ok(())
}

/// Ends the row whose cells were just made: the line and the character of its place follow its cells, and it takes its turn
/// among the rows rendered, as `rule`'s.
pub(crate) fn placed_row(into: &mut Rendered, rule: RuleNumber, place: Point) {
    into.cells.push(line_cell(place));
    into.cells.push(column_cell(place));
    let number = next_row(into);
    end_row(into);
    into.order.push(RowOrder {
        rule,
        line: place.line,
        column: place.column,
        row: number,
    });
}

/// Makes the cells of one row as its shape lays them out, before its line and character: `made` answers the cell of the row's
/// column at an index, its text kept at the end of the text it is handed. One row of the old engine's table and one row the new
/// engine's scan kept are laid out here, and nowhere else.
///
/// # Errors
///
/// What `made` refuses: `NoSuchColumn` for a column the row lacks.
pub(crate) fn shown_cells(
    into: &mut Rendered,
    shape: ShownRow<'_>,
    made: &mut dyn FnMut(
        &mut RenderedText,
        ColumnIndex,
    ) -> Result<Option<Cell<TextRange>>, RowsError>,
) -> Result<(), FinishError> {
    let columns = (0..shape.width.0).map(ColumnPlace).map(column_at);
    match shape.shown {
        Shown::All => {
            for column in columns {
                let cell = made(&mut into.text, column)?;
                into.cells.push(cell);
            }
        }
        Shown::Placed(placed) => {
            let id = keep(&mut into.text, Cell::Text(CellText(placed.rule.0)));
            into.cells.push(Some(id));
            for each in placed.cells {
                let cell = match each {
                    AnswerCell::Column(own) if own.0 < shape.width.0 => {
                        made(&mut into.text, column_at(*own))?
                    }
                    AnswerCell::Column(_) | AnswerCell::Empty => None,
                };
                into.cells.push(cell);
            }
        }
        Shown::Finding(rule) => {
            let emit = shape.emit;
            let id = keep(&mut into.text, Cell::Text(CellText(rule.0)));
            into.cells.push(Some(id));
            let subject = made(&mut into.text, emit.subject)?;
            into.cells.push(subject);
            into.cells.push(Some(key_count(KeyCount(emit.key.len()))));
            for index in &emit.key {
                let cell = made(&mut into.text, *index)?;
                into.cells.push(cell);
            }
            for (heading, column) in emit.headings.iter().zip(columns) {
                let named = keep(&mut into.text, Cell::Text(CellText(&heading.0)));
                into.cells.push(Some(named));
                let cell = made(&mut into.text, column)?;
                into.cells.push(cell);
            }
        }
    }
    Ok(())
}

/// The column at a place among a row's columns.
fn column_at(place: ColumnPlace) -> ColumnIndex {
    ColumnIndex(u32::try_from(place.0).unwrap_or(u32::MAX))
}

/// The cells of the row numbered `row` among the rows whose ends are `ends`: none for a row they do not have.
#[must_use]
pub(crate) fn row_cells<'a, T>(
    cells: &'a [Option<Cell<T>>],
    ends: &RowEnds,
    row: RowNumber,
) -> &'a [Option<Cell<T>>] {
    let at = usize::try_from(row.0).unwrap_or(usize::MAX);
    let bound = |place| {
        ends.0
            .get(place)
            .map(|end| usize::try_from(*end).unwrap_or(usize::MAX))
    };
    let from = at.checked_sub(1).and_then(bound).unwrap_or(0);
    let to = bound(at).unwrap_or(from);
    cells.get(from..to).unwrap_or(&[])
}

/// The cells of a row, each lent with its text from `text`, the text of its rows.
pub(crate) fn lent_row<'a>(
    text: RowsTextView<'a>,
    cells: &'a [Option<Cell<TextRange>>],
) -> impl Iterator<Item = Option<Cell<CellText<'a>>>> + Clone + 'a {
    cells
        .iter()
        .map(move |cell| cell.map(|kept| lent(text, kept)))
}

/// The parts of a finding's row, read back from its cells as [`Shown::Finding`] lays them out and every row ends; none for a row that
/// is not laid out so.
#[must_use]
pub(crate) fn finding_parts<T>(cells: &[Option<Cell<T>>]) -> Option<FindingParts<'_, T>> {
    let (column, rest) = cells.split_last()?;
    let (line, rest) = rest.split_last()?;
    let [
        Some(Cell::Text(rule)),
        subject,
        Some(Cell::Number(count)),
        rest @ ..,
    ] = rest
    else {
        return None;
    };
    let (key, columns) = rest.split_at_checked(usize::try_from(count.0).unwrap_or(usize::MAX))?;
    columns.len().is_multiple_of(2).then_some(FindingParts {
        rule,
        subject,
        key,
        columns,
        line,
        column,
    })
}

use ryo_values::domain::rows::types::RowIndex;
