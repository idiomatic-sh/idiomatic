//! Execute source-keyed closure requests through the ordinary join transport and emit contract.

use super::types::lookup::value::ReachJoin;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext};
use crate::domain::replay::types::Pipe;
use crate::domain::table_model::service::{count_of, positions, read_row};
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::relations::types::Value;
use ryo_values::domain::plans::types::Operation;

/// Computed joins preserve left-row order and duplicate multiplicity without global closure materialization.
pub(crate) fn emit(
    operation: &Operation,
    left: &Rows<'_>,
    right: &Rows<'_>,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let mut row = std::mem::take(&mut context.scratch.row);
    let mut lookup = ReachJoin::new(
        operation,
        right,
        std::mem::take(&mut context.scratch.walk_scratch.graph_walk),
        std::mem::take(&mut context.scratch.join_scratch.key),
        std::mem::take(&mut context.scratch.other),
    )?;
    let result = positions(count_of(left)).try_for_each(|position| {
        if let Some(left) = read_row(left, position, &mut row) {
            lookup.row(left, context, emit)?;
        }
        Ok(())
    });
    let (walk, key, output) = lookup.finish();
    context.scratch.row = row;
    context.scratch.walk_scratch.graph_walk = walk;
    context.scratch.join_scratch.key = key;
    context.scratch.other = output;
    result
}

/// Streamed joins keep preparation lazy until an actual left row, after their original RHS hold.
pub(crate) fn stream(
    operation: &Operation,
    inputs: &mut dyn Feed<'_>,
    right: &Rows<'_>,
    context: &mut OperatorContext<'_>,
    scratch: &mut OperatorScratch,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let mut lookup = ReachJoin::new(
        operation,
        right,
        std::mem::take(&mut scratch.walk_scratch.graph_walk),
        std::mem::take(&mut scratch.join_scratch.key),
        std::mem::take(&mut scratch.other),
    )?;
    let result = inputs.stream(
        InputPlace(0),
        context,
        &mut Pipe(|context: &mut OperatorContext<'_>, left: &[Value]| {
            lookup.row(left, context, emit)
        }),
    );
    let (walk, key, output) = lookup.finish();
    scratch.walk_scratch.graph_walk = walk;
    scratch.join_scratch.key = key;
    scratch.other = output;
    result
}
