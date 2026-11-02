//! The `extend` operator: append columns holding a number or a parameter.

use ryo_language::domain::relations::types::{CellKind, Flag, Integer, Value};
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::terms::types::NameText;

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::first;
use crate::domain::operating::service::minted;
use crate::domain::operating::types::AfterTheFiles;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room};
use crate::domain::replay::types::Pipe;
use crate::domain::table_model::service::{count_of, kinds_of, positions, read_row, reset};
use crate::domain::table_model::types::{ParameterBinding, RowCount, Rows, Table};
use ryo_values::domain::plan_parts::service::parameter_in;
use ryo_values::domain::plan_parts::types::FlagState;
use ryo_values::domain::plan_parts::types::{Operand, QueryLiteral};
use ryo_values::domain::plans::types::Operation;

/// The values an extend appends to every row: numbers, flags, and the names its texts are, made once.
fn appended_values(
    values: &[Operand],
    context: &mut OperatorContext<'_>,
    appended: &mut Vec<Value>,
) -> Result<(), OperatingError> {
    let OperatorContext {
        bindings,
        symbols,
        source,
        arenas,
        ..
    } = context;
    let after = if arenas.is_empty() {
        AfterTheFiles::Before
    } else {
        AfterTheFiles::After
    };
    appended.clear();
    for operand in values {
        appended.push(match operand {
            Operand::Literal(QueryLiteral::Number(number)) => Value::Number(Integer(number.0)),
            // A text becomes a name of the file's names, the same name for the same text.
            Operand::Literal(QueryLiteral::Text(text)) => {
                let made = query_intern_derived(symbols, ByteSlice(source.0), NameText(&text.0));
                minted(symbols.as_ref(), made, after)
            }
            Operand::Column(_) | Operand::Function(_) => return Err(OperatingError::WrongBinding),
            Operand::Parameter(id) => {
                match parameter_in(bindings, *id).ok_or(OperatingError::Unbound)? {
                    ParameterBinding::Number(number) => Value::Number(Integer(number.0)),
                    ParameterBinding::Flag(flag) => Value::Flag(if *flag == FlagState::Raised {
                        Flag::Raised
                    } else {
                        Flag::Lowered
                    }),
                    ParameterBinding::Text(text) => {
                        let made =
                            query_intern_derived(symbols, ByteSlice(source.0), NameText(&text.0));
                        minted(symbols.as_ref(), made, after)
                    }
                    ParameterBinding::Numbers(_)
                    | ParameterBinding::Texts(_)
                    | ParameterBinding::TextSets(_)
                    | ParameterBinding::Tables(_) => {
                        return Err(OperatingError::WrongBinding);
                    }
                }
            }
        });
    }
    Ok(())
}

/// `extend`, one item of the `operators` collection.
pub(crate) struct ExtendItem;

impl OperatorsItem for ExtendItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::Extend { values, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let rows = first(inputs)?;
        let mut appended = std::mem::take(&mut context.scratch.other);
        let mut buffer = std::mem::take(&mut context.scratch.row);
        let made = appended_values(values, context, &mut appended);
        if let Err(error) = made {
            context.scratch.other = appended;
            context.scratch.row = buffer;
            return Err(error);
        }
        reset(out, kinds_of(rows));
        out.kinds.extend(appended.iter().map(|value| match value {
            Value::Flag(_) => CellKind::Flag,
            Value::Symbol(_) | Value::Name(_) => CellKind::Symbol,
            Value::Node(_) | Value::File(_) | Value::Span(_) | Value::Number(Integer(_)) => {
                CellKind::Number
            }
        }));
        for position in positions(count_of(rows)) {
            let Some(row) = read_row(rows, position, &mut buffer) else {
                continue;
            };
            out.cells.extend_from_slice(row);
            out.cells.extend_from_slice(&appended);
            out.count = RowCount(out.count.0 + 1);
        }
        context.scratch.other = appended;
        context.scratch.row = buffer;
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
        let Operation::Extend { values, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let Room { scratch, .. } = room;
        let mut appended = std::mem::take(&mut scratch.other);
        appended.clear();
        let mut joined = std::mem::take(&mut scratch.row);
        let mut made = false;
        let outcome = ins.stream(
            InputPlace(0),
            context,
            &mut Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                // What is appended is made when the first row comes: no row, no names made.
                if !made {
                    appended_values(values, each, &mut appended)?;
                    made = true;
                }
                joined.clear();
                joined.extend_from_slice(row);
                joined.extend_from_slice(&appended);
                emit.row(each, &joined)
            }),
        );
        scratch.other = appended;
        scratch.row = joined;
        outcome
    }
}
