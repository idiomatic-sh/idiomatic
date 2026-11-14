//! The `heads` application operation.

use ryo_execution_domain::domain::interpreted_program::types::{
    LogicalInstruction, instruction::value::Instruction,
    interpreted_program::value::InterpretedProgram,
};
use ryo_relations::domain::relations::{
    service::{ALL, columns},
    types::{Column, Relation},
};
use ryo_values::domain::{
    plan_parts::types::{Input as Source, Operand, RepeatedRows},
    plans::types::Operation,
};
use std::collections::HashSet;

// Every catalog relation has a typed borrowed-row representation. Source loading
// and demand availability are runtime obligations, not static head eligibility.
pub(crate) fn supported_head(instruction: &Instruction) -> Option<Relation> {
    let (source, projected, predicates) = match &instruction.operation() {
        Operation::Select {
            from: Source::Relation(source),
            kept,
            comparisons,
        } => (*source, kept, comparisons.as_slice()),
        Operation::Project {
            from: Source::Relation(source),
            columns,
            distinct: RepeatedRows::Kept,
        } => (*source, columns, &[][..]),
        _ => return None,
    };
    let source_columns = columns(source);
    let width = source_columns.len();
    fn valid(operand: &Operand, columns: &[Column]) -> Option<()> {
        match operand {
            Operand::Column(column) => columns.get(column.0 as usize).map(|_| ()),
            Operand::Function(function) => function
                .arguments
                .iter()
                .try_for_each(|argument| valid(argument, columns)),
            Operand::Parameter(_) | Operand::Literal(_) => Some(()),
        }
    }
    (ALL.contains(&source)
        && projected.iter().all(|column| (column.0 as usize) < width)
        && predicates.iter().all(|predicate| {
            valid(&predicate.left, &source_columns).is_some()
                && valid(&predicate.right, &source_columns).is_some()
        }))
    .then_some(source)
}

pub(crate) fn expected(program: &InterpretedProgram) -> HashSet<LogicalInstruction> {
    program
        .regions()
        .iter()
        .filter_map(|region| region.instructions().first())
        .filter(|instruction| supported_head(instruction).is_some())
        .map(|instruction| instruction.logical())
        .collect()
}
