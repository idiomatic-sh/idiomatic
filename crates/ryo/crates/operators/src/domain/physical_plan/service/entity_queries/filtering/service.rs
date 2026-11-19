//! Bind a filter only when each comparison is total on the registered source schema.

use super::super::projection::service::columns;
use crate::domain::entity_queries::{
    service,
    types::{
        EntityAttribute as Field, EntityColumn, EntityExportId, EntityPredicate as Test,
        EntityQueryOp as Operation, EntityStage as Stage,
    },
};
use crate::domain::physical_plan::types::{IndexKey, KeyedIndex, PlanNodeId, RowSource};
use crate::domain::select::types::SelectParts;
use ryo_language::domain::symbols::service::hash_of;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::TextHash;
use ryo_values::domain::plan_parts::types::{
    Operand, PlanComparison, QueryLiteral, StepId, Test as Comparison,
};

pub(in super::super) fn lookup(
    index: &KeyedIndex,
    key: TextHash,
    select: &SelectParts,
) -> Option<RowSource> {
    let (RowSource::Facts(_), IndexKey::Text(column)) = (index.source, &index.key) else {
        return None;
    };
    select
        .comparisons
        .iter()
        .any(|comparison| {
            matches!((&comparison.left, &comparison.right),
            (Operand::Column(left), Operand::Literal(QueryLiteral::Text(text)))
                if comparison.test == Comparison::Equal && left == column && !text.0.is_empty()
                    && hash_of(ByteSlice(text.0.as_bytes())) == key)
        })
        .then_some(index.source)
}

pub(in super::super) fn filter(
    fields: &[EntityColumn],
    select: &SelectParts,
    input: StepId,
    physical: PlanNodeId,
) -> Option<Stage> {
    if !select.bindings.is_empty() {
        return None;
    }
    let tests = select
        .comparisons
        .iter()
        .map(|test| predicate(fields, test))
        .collect::<Option<_>>()?;
    let fields = if select.kept.is_empty() {
        fields.to_vec()
    } else {
        columns(fields, &select.kept)?
    };
    Some(Stage {
        expr: Operation::Filter { tests },
        fields,
        operands: vec![input],
        physical: Some(EntityExportId(physical.0)),
    })
}

fn predicate(fields: &[EntityColumn], test: &PlanComparison) -> Option<Test> {
    match test.test {
        Comparison::Equal | Comparison::NotEqual => (),
        _ => return None,
    };
    let Operand::Column(left) = &test.left else {
        return None;
    };
    let column = usize::from(*left);
    let EntityColumn(field, _) = *fields.get(column)?;
    match &test.right {
        Operand::Literal(QueryLiteral::Text(text))
            if matches!(field, Field::Kind | Field::Text | Field::Role) =>
        {
            Some(service::literal(*left, test.test, text))
        }
        Operand::Column(right) if matches!(field, Field::Id | Field::Ordinal | Field::File) => {
            let peer = usize::from(*right);
            (field == fields.get(peer)?.0).then(|| service::pair(*left, test.test, *right))
        }
        _ => None,
    }
}
