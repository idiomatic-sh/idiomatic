//! The `rank` operator: append the place of the first of an ordered list of sets of text that holds the text of a column.

use ryo_language::domain::relations::types::{CellKind, Integer, Value};

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::first;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room};
use crate::domain::replay::service::derive;
use crate::domain::replay::types::{Collect, Pipe};
use crate::domain::table_model::service::{cell, count_of, kinds_of, positions, read_row, reset};
use crate::domain::table_model::types::{ParameterBinding, Rows, Table};
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_values::domain::plan_parts::service::parameter_in;
use ryo_values::domain::plans::types::Operation;

/// `rank`, one item of the `operators` collection.
pub(crate) struct RankItem;

/// Passes on `row` with the place of the first set that holds its text; a row whose text no set holds is not passed on.
fn rank_row(
    operation: &Operation,
    row: &[Value],
    context: &mut OperatorContext<'_>,
    joined: &mut Vec<Value>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let Operation::Rank { column, sets, .. } = operation else {
        return Err(OperatingError::WrongOperation);
    };
    let ParameterBinding::TextSets(layers) =
        parameter_in(context.bindings, *sets).ok_or(OperatingError::Unbound)?
    else {
        return Err(OperatingError::WrongBinding);
    };
    let OperatorContext {
        symbols,
        source,
        paths,
        arenas,
        ..
    } = &*context;
    let texts = Texts {
        symbols: symbols.as_ref(),
        source: *source,
        paths,
        arenas,
    };
    let Datum::Text(named) = datum(cell(row, *column)?, &texts) else {
        return Ok(());
    };
    let Some(place) = layers
        .iter()
        .position(|set| set.iter().any(|each| *each.0 == *named.0))
    else {
        return Ok(());
    };
    joined.clear();
    joined.extend_from_slice(row);
    joined.push(Value::Number(Integer(
        i64::try_from(place).unwrap_or(i64::MAX),
    )));
    emit.row(context, joined)
}

impl OperatorsItem for RankItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::Rank { .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let rows = first(inputs)?;
        reset(out, kinds_of(rows));
        out.kinds.push(CellKind::Number);
        let mut buffer = std::mem::take(&mut context.scratch.row);
        let mut joined = std::mem::take(&mut context.scratch.other);
        let outcome = (|| {
            let mut emit = Collect(out);
            for position in positions(count_of(rows)) {
                let Some(row) = read_row(rows, position, &mut buffer) else {
                    continue;
                };
                rank_row(operation, row, context, &mut joined, &mut emit)?;
            }
            Ok(())
        })();
        context.scratch.row = buffer;
        context.scratch.other = joined;
        outcome
    }

    fn stream(
        &self,
        operation: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let Room { memo, scratch, .. } = room;
        let mut joined = std::mem::take(&mut scratch.other);
        let outcome = ins.stream(
            InputPlace(0),
            context,
            &mut Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                let mut own = derive(each, memo, scratch);
                rank_row(operation, row, &mut own, &mut joined, emit)
            }),
        );
        scratch.other = joined;
        outcome
    }
}
