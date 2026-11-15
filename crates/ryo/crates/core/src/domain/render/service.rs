//! The `render` service: free functions named by verb that turn the rows of one file into typed cells and
//! let the files leave in order, with no IO and no ports. A name's text is kept here, one row at a
//! time, as the row is made; no cell becomes text for a reader in the core.

use ryo_language::domain::elements::service::start_at;
use ryo_language::domain::names::service::places_of;
use ryo_language::domain::symbols::service::place_of;
use ryo_relations::domain::relations::service::{node_file, node_position};
use ryo_relations::domain::relations::types::{Integer, Name, NodeId, Value};

use super::constants::rule_heading;
use super::types::{
    AnswerCell, AnswerColumns, Answering, ColumnPlace, FindingParts, FindingRule, KeyCount,
    KeyOffset, RenderInput, Rendered, RowKey, RowLayout, RowLimit, RowNumber, RowOrder, RowsAnswer,
    RuleRow, Shown, TableRow,
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
use crate::domain::value_cells::types::Scene;
use ryo_values::domain::plan_parts::types::Heading;
use ryo_values::domain::plans::types::{Emit, Plan};
use ryo_values::domain::records::service::{keep, lent};
use ryo_values::domain::row_texts::types::{
    Cell, CellText, RowEnds, RowTotal, RowsTextView, TextRange,
};
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
        match layout.shown {
            Shown::All => {
                for value in row {
                    let made = value_cell(&mut into.text, *value, scene);
                    into.cells.push(made);
                }
            }
            Shown::Placed(placed) => render_placed(into, placed, row, scene),
            Shown::Finding(finding_rule) => render_finding(into, finding_rule, row, emit, scene)?,
        }
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
    into.keys = kept;
    Ok(())
}

/// Makes the cells of one rule's row among the columns of the answer: the rule's id, then each column of the answer, the rule's own
/// or empty.
fn render_placed(into: &mut Rendered, placed: RuleRow<'_>, row: &[Value], scene: Scene<'_>) {
    let id = keep(&mut into.text, Cell::Text(CellText(placed.rule.0)));
    into.cells.push(Some(id));
    for each in placed.cells {
        let made = match each {
            AnswerCell::Column(column) => row
                .get(column.0)
                .and_then(|value| value_cell(&mut into.text, *value, scene)),
            AnswerCell::Empty => None,
        };
        into.cells.push(made);
    }
}

/// Makes the cells of one finding's row, as [`Shown::Finding`] lays them out: the id of its rule, its subject, how many key columns it
/// has and the value of each, then each of its columns as its heading and its value.
fn render_finding(
    into: &mut Rendered,
    rule: FindingRule<'_>,
    row: &[Value],
    emit: &Emit,
    scene: Scene<'_>,
) -> Result<(), FinishError> {
    let id = keep(&mut into.text, Cell::Text(CellText(rule.0)));
    into.cells.push(Some(id));
    let subject = value_cell(&mut into.text, cell(row, emit.subject)?, scene);
    into.cells.push(subject);
    into.cells.push(Some(key_count(KeyCount(emit.key.len()))));
    for index in &emit.key {
        let made = value_cell(&mut into.text, cell(row, *index)?, scene);
        into.cells.push(made);
    }
    for (heading, value) in emit.headings.iter().zip(row) {
        let named = keep(&mut into.text, Cell::Text(CellText(&heading.0)));
        into.cells.push(Some(named));
        let made = value_cell(&mut into.text, *value, scene);
        into.cells.push(made);
    }
    Ok(())
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
