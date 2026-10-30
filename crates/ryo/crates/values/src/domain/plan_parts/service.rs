//! The `plan_parts` service: free functions named by verb over the numbers a plan is made of, with no IO and no ports.

use super::types::{Applied, ColumnIndex, Operand, ParameterId, StepId};

/// The one of `items` that the step `id` names, when there is one.
#[must_use]
pub fn step_in<T>(items: &[T], id: StepId) -> Option<&T> {
    items.get(usize::try_from(id.0).unwrap_or(usize::MAX))
}

/// The one of `items` that the parameter `id` names, when there is one.
#[must_use]
pub fn parameter_in<T>(items: &[T], id: ParameterId) -> Option<&T> {
    items.get(usize::try_from(id.0).unwrap_or(usize::MAX))
}

/// Calls `visit` with each column an operand reads: the column itself, or the columns the arguments of its function read.
pub fn each_column<F>(operand: &Operand, visit: &mut F)
where
    F: FnMut(ColumnIndex),
{
    match operand {
        Operand::Column(column) => visit(*column),
        Operand::Function(applied) => {
            for argument in &applied.arguments {
                each_column(argument, visit);
            }
        }
        Operand::Parameter(_) | Operand::Literal(_) => {}
    }
}

/// Calls `visit` with each column an operand reads, to be renamed.
pub fn each_column_mut<F>(operand: &mut Operand, visit: &mut F)
where
    F: FnMut(&mut ColumnIndex),
{
    match operand {
        Operand::Column(column) => visit(column),
        Operand::Function(applied) => {
            for argument in &mut applied.arguments {
                each_column_mut(argument, visit);
            }
        }
        Operand::Parameter(_) | Operand::Literal(_) => {}
    }
}

/// The columns an operand reads, itself or through the arguments of its function, in the order written.
#[must_use]
pub fn columns_of(operand: &Operand) -> Vec<ColumnIndex> {
    let mut read = Vec::new();
    each_column(operand, &mut |column| read.push(column));
    read
}

/// The operand with each column it reads put where `remap` says; nothing when `remap` has no place for one of them.
pub fn remapped<F>(operand: &Operand, remap: &F) -> Option<Operand>
where
    F: Fn(ColumnIndex) -> Option<ColumnIndex>,
{
    match operand {
        Operand::Column(column) => remap(*column).map(Operand::Column),
        Operand::Function(applied) => Some(Operand::Function(Applied {
            function: applied.function,
            arguments: applied
                .arguments
                .iter()
                .map(|argument| remapped(argument, remap))
                .collect::<Option<Vec<_>>>()?,
        })),
        other @ (Operand::Parameter(_) | Operand::Literal(_)) => Some(other.clone()),
    }
}
