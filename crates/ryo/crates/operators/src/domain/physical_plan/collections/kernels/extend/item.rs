//! The `extend` item of the `kernels` collection: each row followed by the values of an extension, resolved once per file: a
//! written number, a written text minted as a name of the file, or what a parameter is bound to.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use crate::domain::contract::types::PortableOperatorFault;
#[cfg(test)]
use crate::domain::physical_plan::types::Lowering;
use crate::domain::physical_plan::types::Resources;
use ryo_relations::domain::relations::types::{CellKind, Flag, Integer, Value};
use ryo_values::domain::plan_parts::service::parameter_in;
use ryo_values::domain::plan_parts::types::{FlagState, Operand, QueryLiteral};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::{RowCount, Table};

/// `extend`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct ExtendItem;

#[cfg(test)]
impl KernelsItem for ExtendItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = { matches!(lowering, Lowering::Extend { .. }) };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// The values of an extension, resolved into `into` in order.
///
/// # Errors
///
/// `Unbound` for a parameter with no binding; `WrongBinding` for a column, a function, or a parameter bound to more than one
/// value.
pub(crate) fn resolved(
    values: &[Operand],
    bindings: &[ParameterBinding],
    resources: &mut dyn Resources,
    into: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    into.clear();
    for operand in values {
        into.push(match operand {
            Operand::Literal(QueryLiteral::Number(number)) => Value::Number(Integer(number.0)),
            // A text becomes a name of the file's names, the same name for the same text.
            Operand::Literal(QueryLiteral::Text(text)) => resources.mint(CellText(&text.0)),
            Operand::Column(_) | Operand::Function(_) => {
                return Err(PortableOperatorFault::WrongBinding);
            }
            Operand::Parameter(id) => {
                match parameter_in(bindings, *id).ok_or(PortableOperatorFault::Unbound)? {
                    ParameterBinding::Number(number) => Value::Number(Integer(number.0)),
                    ParameterBinding::Flag(flag) => Value::Flag(if *flag == FlagState::Raised {
                        Flag::Raised
                    } else {
                        Flag::Lowered
                    }),
                    ParameterBinding::Text(text) => resources.mint(CellText(&text.0)),
                    ParameterBinding::Numbers(_)
                    | ParameterBinding::Texts(_)
                    | ParameterBinding::TextSets(_)
                    | ParameterBinding::Tables(_) => {
                        return Err(PortableOperatorFault::WrongBinding);
                    }
                }
            }
        });
    }
    Ok(())
}

/// What a column holding `value` holds.
pub(crate) fn kind_of(value: Value) -> CellKind {
    match value {
        Value::Node(_) => CellKind::Node,
        Value::Symbol(_) | Value::Name(_) => CellKind::Symbol,
        Value::File(_) => CellKind::File,
        Value::Span(_) => CellKind::Span,
        Value::Number(_) => CellKind::Number,
        Value::Flag(_) => CellKind::Flag,
    }
}

/// Appends `row` followed by `appended` to `output`, written through the extension's one meaning into `written`.
pub(crate) fn extended(
    row: &[Value],
    appended: &[Value],
    written: &mut Vec<Value>,
    output: &mut Table,
) {
    crate::domain::append::service::append(row, appended, written);
    output.cells.extend_from_slice(written);
    output.count = RowCount(output.count.0 + 1);
}
