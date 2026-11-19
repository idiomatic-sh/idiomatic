//! Total schema and literal operations shared by the file and planner.
use super::types::{EntityAttribute, EntityLiteralCode, EntityOperand, EntityPredicate};
use crate::domain::table_texts::{
    service::symbol_text,
    types::{TextView, Texts},
};
use ryo_relations::domain::{
    relations::{
        constants::NO_SYMBOL,
        types::{CellKind, ColumnsView},
    },
    terms::types::Symbol,
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, LiteralText, Test},
    rows::types::RowCount,
};
/// Preserve the ordinary physical kind of this source field.
pub(super) fn kind(field: EntityAttribute) -> CellKind {
    match field {
        EntityAttribute::Id => CellKind::Node,
        EntityAttribute::Kind | EntityAttribute::Text | EntityAttribute::Role => CellKind::Symbol,
        EntityAttribute::File => CellKind::File,
        EntityAttribute::Span => CellKind::Span,
        EntityAttribute::Ordinal => CellKind::Number,
    }
}
/// Obtain a stable spelling for every future answer on the open file. A forward symbol can acquire bytes later.
pub(super) fn spelling<'a>(texts: &Texts<'a>, symbol: Symbol) -> Option<TextView<'a>> {
    match symbol_text(texts, symbol) {
        Some(found) => Some(found),
        None if symbol == NO_SYMBOL => Some(TextView::default()),
        None => None,
    }
}
/// Only complete, rectangular columns of their declared physical schema can support an entity file.
pub(super) fn shape(view: ColumnsView<'_>, kinds: &[CellKind]) -> Option<RowCount> {
    let n = view.columns.first()?.0.len();
    (view.kinds == kinds
        && view.columns.len() == kinds.len()
        && view.columns.iter().all(|col| col.0.len() == n))
    .then_some(RowCount(n))
}
/// A checked literal receives its stable small code while preparing all file queries.
pub(crate) fn literal(column: ColumnIndex, test: Test, text: &LiteralText) -> EntityPredicate {
    EntityPredicate {
        column,
        test,
        operand: EntityOperand::Text(text.clone(), EntityLiteralCode(u32::MAX)),
    }
}
/// An equality or inequality between comparable fixed-width values.
pub(crate) fn pair(column: ColumnIndex, test: Test, peer: ColumnIndex) -> EntityPredicate {
    EntityPredicate {
        column,
        test,
        operand: EntityOperand::Column(peer),
    }
}

/// Compile sibling subscriptions through the same predicate planner used by packet execution.
pub(super) fn entity_partitions(
    stages: &[super::types::EntityStage],
) -> Option<std::sync::Arc<crate::domain::row_predicates::types::Program>> {
    use super::types::{EntityOperand, EntityQueryOp};
    use crate::domain::row_predicates::types::{
        Equality, Instruction, LiteralId, PredicateSelection, SelectionInput,
    };
    let selections = stages
        .iter()
        .map(|stage| {
            let EntityQueryOp::Filter { tests } = &stage.expr else {
                return PredicateSelection::default();
            };
            let instructions = tests
                .iter()
                .filter_map(|test| match (&test.operand, test.test) {
                    (
                        EntityOperand::Text(_, code),
                        ryo_values::domain::plan_parts::types::Test::Equal,
                    ) => Some(Instruction::Word(
                        test.column,
                        LiteralId(code.0 as usize),
                        Equality::Equal,
                    )),
                    _ => None,
                })
                .collect();
            PredicateSelection {
                source: SelectionInput::Derived(stage.operands[0]),
                instructions,
            }
        })
        .collect();
    let program = crate::domain::row_predicates::service::prepare(selections);
    (!program.partitions.is_empty()).then(|| std::sync::Arc::new(program))
}
