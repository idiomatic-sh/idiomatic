//! Append one literal regex replacement result per input row.

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::{first, minted};
use crate::domain::operating::types::{AfterTheFiles, Emit, Feed, OperatorContext, Room};
use crate::domain::replay::service::replay_computed;
use crate::domain::table_model::service::{cell, count_of, kinds_of, positions, read_row, reset};
use crate::domain::table_model::types::{Rows, Table};
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, TextView, Texts};
use ryo_language::domain::relations::types::CellKind;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::terms::types::NameText;
use ryo_values::domain::plans::types::Operation;

/// One validated replacement projection.
pub(crate) struct ReplaceItem;

impl OperatorsItem for ReplaceItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::Replace {
            text,
            pattern,
            replacement,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let input = first(inputs)?;
        reset(out, kinds_of(input));
        out.kinds.push(CellKind::Symbol);
        let mut buffer = Vec::new();
        for at in positions(count_of(input)) {
            let Some(row) = read_row(input, at, &mut buffer) else {
                continue;
            };
            let texts = Texts {
                symbols: context.symbols,
                source: context.source,
                paths: context.paths,
                arenas: context.arenas,
            };
            let Datum::Text(value) = datum(cell(row, *text)?, &texts) else {
                return Err(OperatingError::ShapeMismatch);
            };
            let changed = crate::domain::matching::service::replace(
                value,
                TextView(&pattern.0),
                TextView(&replacement.0),
                context.prepared_matchers,
                &mut context.scratch.select_scratch.compiled_regexes,
            )?;
            let made = intern_derived(
                context.symbols,
                ByteSlice(context.source.0),
                NameText(&changed.0),
            );
            let after = if context.arenas.is_empty() {
                AfterTheFiles::Before
            } else {
                AfterTheFiles::After
            };
            out.cells.extend_from_slice(row);
            out.cells.push(minted(context.symbols, made, after));
            out.count.0 += 1;
        }
        Ok(())
    }

    fn stream(
        &self,
        operation: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        replay_computed(
            operation,
            ins,
            context,
            room,
            emit,
            |operation, inputs, context, out| ReplaceItem.compute(operation, inputs, context, out),
        )
    }
}
