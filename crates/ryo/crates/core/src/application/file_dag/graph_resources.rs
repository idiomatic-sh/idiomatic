//! Behavioral controls retained from the former application resource constructors.
use crate::domain::{
    derived_views::types::*, operating::errors::OperatingError,
    operation_schema::types::InputSchema, operator::contract::views::BorrowedRows,
    table_model::service::push_row,
};
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
fn prepare_cycles(
    input: BorrowedRows<'_>,
    source: ColumnIndex,
    destination: ColumnIndex,
) -> Result<super::derived::view::AcquiredView, OperatingError> {
    let owner =
        super::relations::RelationOwner::new(true, crate::outbound::derived::factory::test_port());
    let proof = owner.admit_external(input)?;
    let (population, _) = proof.identity(&owner)?;
    let plan = std::sync::Arc::new(super::derived::prepare::view(
        owner.derived.semantic_port(),
        &ViewRequest::CycleAdjacency {
            source,
            destination,
        },
        &[InputSchema {
            kinds: input.kinds(),
            tree: None,
        }],
    )?);
    owner.derived.acquire_stored(
        &plan,
        &[population],
        super::derived::owner::ViewContext::default(),
        DerivedInputs::Rows(&[proof.rows()]),
        &mut DerivedScratch::default(),
        None,
    )
}
fn prepare_closure(
    input: BorrowedRows<'_>,
    keys: &[ColumnIndex],
    source: ColumnIndex,
    destination: ColumnIndex,
) -> Result<Vec<super::derived::view::AcquiredView>, OperatingError> {
    let mut cursor =
        super::closure_partitions::PartitionedClosure::prepare(input, keys, source, destination)?;
    let mut parts = Vec::new();
    while cursor.advance()? {
        parts.push(cursor.take_current()?);
    }
    Ok(parts)
}
#[cfg(test)]
mod tests {
    use super::*;
    use ryo_relations::domain::relations::types::Integer;
    fn number(value: i64) -> Value {
        Value::Number(Integer(value))
    }
    fn table(width: usize, rows: &[&[i64]]) -> Table {
        let mut table = Table {
            kinds: vec![CellKind::Number; width],
            ..Table::default()
        };
        for row in rows {
            push_row(
                &mut table,
                &row.iter().copied().map(number).collect::<Vec<_>>(),
            );
        }
        table
    }
    #[test]
    fn adjacency_keeps_parallel_edges_and_first_entity_numbering() -> Result<(), OperatingError> {
        let input = table(2, &[&[9, 1], &[9, 1], &[1, 9], &[7, 7]]);
        let graph = prepare_cycles(
            BorrowedRows::Computed(&input),
            ColumnIndex(0),
            ColumnIndex(1),
        )
        .unwrap();
        let ryo_derived::domain::contract::types::DerivedView::CycleAdjacency(graph) =
            graph.view()?
        else {
            return Err(OperatingError::WrongOperation);
        };
        assert_eq!(graph.values(), vec![number(9), number(1), number(7)]);
        assert_eq!(graph.starts(), vec![0, 2, 3, 4]);
        assert_eq!(graph.leaving(), vec![1, 1, 0, 2]);
        Ok(())
    }
    #[test]
    fn closure_preparation_preserves_clustered_partition_order_and_sorts_sources()
    -> Result<(), OperatingError> {
        let input = table(3, &[&[9, 4, 3], &[9, 2, 4], &[1, 7, 8]]);
        let partitions = prepare_closure(
            BorrowedRows::Computed(&input),
            &[ColumnIndex(0)],
            ColumnIndex(1),
            ColumnIndex(2),
        )
        .unwrap();
        assert_eq!(partitions.len(), 2);
        use crate::domain::operator::contract::views::ClosureParts;
        let first = ClosureParts::get(&partitions, 0)?;
        let second = ClosureParts::get(&partitions, 1)?;
        assert_eq!(first.key(), vec![number(9)]);
        assert_eq!(first.sources(), vec![number(2), number(4)]);
        assert_eq!(
            first.table().cells,
            vec![number(4), number(3), number(2), number(4)]
        );
        assert_eq!(second.key(), vec![number(1)]);
        Ok(())
    }
}

use ryo_values::domain::rows::types::Table;

use ryo_derived::domain::contract::types::DerivedKind;
