//! Behavioral controls retained from the former application resource constructors.
use crate::domain::{
    derived_views::{collections::views::registration::registered, types::*},
    operating::errors::OperatingError,
    operation_schema::types::InputSchema,
    operator::contract::views::BorrowedRows,
    table_model::{service::push_row, types::Table},
};
use ryo_language::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
fn prepare_cycles(
    input: BorrowedRows<'_>,
    source: ColumnIndex,
    destination: ColumnIndex,
) -> Result<crate::domain::operator::contract::graph::CycleGraph, OperatingError> {
    let plan = registered(DerivedKind::CycleAdjacency).prepare(
        &ViewRequest::CycleAdjacency {
            source,
            destination,
        },
        &[InputSchema {
            kinds: input.kinds(),
            tree: None,
        }],
    )?;
    let mut storage = DerivedStorage::Unbuilt;
    super::derived::owner::build_scoped(
        crate::application::file_dag::route_counts::BuildRoute::Fixture,
        &plan,
        DerivedInputs::Rows(&[input]),
        &mut storage,
        &mut DerivedScratch::default(),
    )?;
    let DerivedStorage::CycleAdjacency(graph) = storage else {
        return Err(OperatingError::WrongOperation);
    };
    Ok(graph)
}
struct PreparedClosure {
    parts: Vec<crate::domain::operator::contract::graph::ClosurePartition>,
    _leases: Vec<std::sync::Arc<super::derived::ledger::ResourceLease>>,
}
impl std::ops::Deref for PreparedClosure {
    type Target = [crate::domain::operator::contract::graph::ClosurePartition];
    fn deref(&self) -> &Self::Target {
        &self.parts
    }
}
fn prepare_closure(
    input: BorrowedRows<'_>,
    keys: &[ColumnIndex],
    source: ColumnIndex,
    destination: ColumnIndex,
) -> Result<PreparedClosure, OperatingError> {
    let mut cursor =
        super::closure_partitions::PartitionedClosure::prepare(input, keys, source, destination)?;
    let mut parts = Vec::new();
    let mut leases = Vec::new();
    while cursor.advance()? {
        let (part, lease) = cursor.take_current()?;
        parts.push(part);
        leases.push(lease);
    }
    Ok(PreparedClosure {
        parts,
        _leases: leases,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use ryo_language::domain::relations::types::Integer;
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
    fn adjacency_keeps_parallel_edges_and_first_entity_numbering() {
        let input = table(2, &[&[9, 1], &[9, 1], &[1, 9], &[7, 7]]);
        let graph = prepare_cycles(
            BorrowedRows::Computed(&input),
            ColumnIndex(0),
            ColumnIndex(1),
        )
        .unwrap();
        assert_eq!(graph.values, vec![number(9), number(1), number(7)]);
        assert_eq!(graph.starts, vec![0, 2, 3, 4]);
        assert_eq!(graph.leaving, vec![1, 1, 0, 2]);
    }
    #[test]
    fn closure_preparation_preserves_clustered_partition_order_and_sorts_sources() {
        let input = table(3, &[&[9, 4, 3], &[9, 2, 4], &[1, 7, 8]]);
        let partitions = prepare_closure(
            BorrowedRows::Computed(&input),
            &[ColumnIndex(0)],
            ColumnIndex(1),
            ColumnIndex(2),
        )
        .unwrap();
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0].key, vec![number(9)]);
        assert_eq!(partitions[0].sources, vec![number(2), number(4)]);
        assert_eq!(
            partitions[0].table.cells,
            vec![number(4), number(3), number(2), number(4)]
        );
        assert_eq!(partitions[1].key, vec![number(1)]);
    }
}
