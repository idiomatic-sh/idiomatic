//! The services of the `matches` part: an ordered match is one made table, filled from the two inputs' rows. A row of it
//! is a pair, the left row's cells and then the right row's (the contract's `OrderedFill::Match`); the operation's answer
//! names columns of either side, and each is read from the pair by its place (`lowered::service::match_column`).

use crate::domain::lowered::service::match_column;
use crate::domain::lowered::types::{Emptiness, FillInput, MadeFill, OrderedFill};
use crate::domain::lowering::definitions::service::fresh_cells;
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::emptiness::service::derived;
use crate::domain::lowering::flattening::service::made_column;
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, RowColumn, RowsPerFile,
};
use crate::domain::lowering::nodes::service::input_shape;
use crate::domain::lowering::nodes::types::StepShape;
use crate::domain::lowering::tables::fills::service::{column_list, fill_of, made_once, scanned};
use crate::domain::lowering::tables::inputs::service::{input, rule};
use crate::domain::lowering::tables::types::{InputWant, MatchSide, RuleFlattening};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, StepId};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::table_words::types::ColumnCount;

/// One side of an ordered match as the fill's input: every row of the side's definition, with its keys and its order.
fn side(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    from: Input,
    keys: &[ColumnIndex],
    order: ColumnIndex,
) -> Option<MatchSide> {
    let read = input(state, drafts, from, InputWant::Defined)?;
    let Emptiness::Definition(definition) = read.emptiness else {
        return None;
    };
    let rows = FillInput {
        pipelines: [scanned(drafts, definition)].into(),
        keys: column_list(keys.iter().map(|key| RowColumn(usize::from(*key)))),
        order: Some(made_column(RowColumn(usize::from(order)))?),
    };
    Some(MatchSide { definition, rows })
}

/// The branch of an ordered match: one row of its table of pairs, the answer's columns read from the pair by place.
pub(crate) fn match_arm(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    step: StepId,
    operation: &Operation,
) -> Option<Flattened> {
    let Operation::OrderedMatch {
        left,
        right,
        left_on,
        right_on,
        left_order,
        right_order,
        output,
    } = operation
    else {
        return None;
    };
    let shapes = &rule(state)?.shapes;
    let mut pair = input_shape(shapes, *left)?;
    let left_width = ColumnCount(pair.0.len());
    pair.0.extend(input_shape(shapes, *right)?.0);
    let lefts = side(state, drafts, *left, left_on, *left_order)?;
    let rights = side(state, drafts, *right, right_on, *right_order)?;
    let parts = vec![
        Emptiness::Definition(lefts.definition),
        Emptiness::Definition(rights.definition),
    ];
    let of = fill_of(state, step, lefts.definition)?;
    let fill = MadeFill::Ordered(OrderedFill::Match(lefts.rows, rights.rows));
    let table = made_once(drafts, of, fill, ColumnCount(pair.0.len()));
    let cells = fresh_cells(drafts, &StepShape(pair.0));
    let mut row = Vec::with_capacity(output.len());
    for column in output {
        let within = made_column(RowColumn(usize::from(column.column)))?;
        let place = match_column(left_width, column.side, within)?;
        row.push(*cells.get(usize::from(place.0))?);
    }
    Some(Flattened {
        branches: vec![FlatBranch {
            atoms: vec![FlatAtom::Rows {
                rows: AtomRows::Made(table),
                cells,
                per_file: RowsPerFile::Many,
            }],
            row,
        }],
        emptiness: derived(operation, parts),
        keyless: Vec::new(),
    })
}
