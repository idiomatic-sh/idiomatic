//! Behavioral controls retained from the former application resource constructors.
use ryo_execution_store::application::file_dag::relations::interface::RelationStore as _;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use {
    ryo_execution_domain::domain::derived_views::types::*,
    ryo_execution_domain::domain::operating::errors::OperatingError,
    ryo_execution_domain::domain::operation_schema::types::InputShape,
    ryo_values::domain::rows::service::push_row,
};
fn prepare_cycles(
    input: RepresentationInput<'_>,
    source: ColumnIndex,
    destination: ColumnIndex,
) -> Result<ryo_execution_store::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView, OperatingError>{
    let owner = ryo_execution_store::application::file_dag::relations::RelationOwner::fixture(
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        (std::sync::Arc::new(ryo_execution_store::application::file_dag::fixtures::derived())
            as std::sync::Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort>),
    );
    let RepresentationInput::Computed(table) = input else {
        panic!("computed fixture")
    };
    let publication = owner.publish(ryo_execution_store::application::file_dag::relations::publication::types::Body::Computed(table.clone()))?;
    let held = owner.rows(&publication, &|_| Err(OperatingError::NotRead))?;
    let input = RepresentationInput::from(&held);
    let proof = owner.admit(&publication, input).and_then(|admitted| admitted)?;
    let plan = std::sync::Arc::new(owner.derived().prepare_view(
        &ViewRequest::CycleAdjacency {
            source,
            destination,
        },
        &[InputShape {
            kinds: input.kinds(),
            tree: None,
        }],
    )?);
    let admitted = owner.derived().admit_view(&plan, &owner, &[&proof])?;
    owner.derived().acquire_stored(
        &plan,
        &admitted,
        ryo_execution_store::application::file_dag::derived::owner::ViewContext::default(),
        DerivedInputs::Rows(&[proof.rows()]),
        &mut DerivedScratch::default(),
        None,
    )
}
fn prepare_closure(
    input: RepresentationInput<'_>,
    keys: &[ColumnIndex],
    source: ColumnIndex,
    destination: ColumnIndex,
) -> Result<crate::application::file_dag::invocation::AcquiredPartitions, OperatingError> {
    let owner = std::rc::Rc::new(ryo_execution_store::application::file_dag::relations::RelationOwner::fixture(
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        (std::sync::Arc::new(
            ryo_execution_store::application::file_dag::fixtures::derived(),
        ) as std::sync::Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort>),
    ));
    let RepresentationInput::Computed(table) = input else {
        return Err(OperatingError::WrongOperation);
    };
    let publication = owner.publish(ryo_execution_store::application::file_dag::relations::publication::types::Body::Computed(table.clone()))?;
    let rows = owner.rows(&publication, &|_| Err(OperatingError::NotRead))?;
    let input = RepresentationInput::from(&rows);
    let mut cursor =
        ryo_execution_store::application::file_dag::closure_partitions::schemas::prepare(
            &publication,
            input,
            keys,
            source,
            destination,
            owner,
        )?;
    let mut parts = crate::application::file_dag::invocation::AcquiredPartitions::default();
    while cursor.advance()? == ryo_execution_store::application::file_dag::closure_partitions::types::PartitionAdvance::Advanced {
        parts.push(cursor.take_current()?);
    }
    Ok(parts)
}
#[cfg(test)]
mod tests {
    use super::*;
    use ryo_relations::domain::relations::types::Integer;
    use ryo_values::domain::table_words::types::ColumnCount;
    fn number(value: Integer) -> Value {
        Value::Number(value)
    }
    fn table(width: ColumnCount, rows: &[&[Integer]]) -> Table {
        let mut table = Table {
            kinds: vec![CellKind::Number; width.0],
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
        let input = table(
            ColumnCount(2),
            &[
                &[Integer(9), Integer(1)],
                &[Integer(9), Integer(1)],
                &[Integer(1), Integer(9)],
                &[Integer(7), Integer(7)],
            ],
        );
        let graph = prepare_cycles(
            RepresentationInput::Computed(&input),
            ColumnIndex(0),
            ColumnIndex(1),
        )
        .unwrap();
        let ryo_derived::domain::contract::types::DerivedView::CycleAdjacency(graph) =
            graph.view()?
        else {
            return Err(OperatingError::WrongOperation);
        };
        assert_eq!(
            graph.values,
            vec![number(Integer(9)), number(Integer(1)), number(Integer(7))]
        );
        assert_eq!(graph.starts.0, vec![0, 2, 3, 4]);
        assert_eq!(graph.leaving.0, vec![1, 1, 0, 2]);
        Ok(())
    }
    #[test]
    fn closure_preparation_preserves_clustered_partition_order_and_sorts_sources()
    -> Result<(), OperatingError> {
        let input = table(
            ColumnCount(3),
            &[
                &[Integer(9), Integer(4), Integer(3)],
                &[Integer(9), Integer(2), Integer(4)],
                &[Integer(1), Integer(7), Integer(8)],
            ],
        );
        let partitions = prepare_closure(
            RepresentationInput::Computed(&input),
            &[ColumnIndex(0)],
            ColumnIndex(1),
            ColumnIndex(2),
        )
        .unwrap();
        use ryo_execution_domain::domain::operator::contract::views::ClosureParts;
        assert_eq!(
            ClosureParts::len(&partitions),
            ryo_execution_domain::domain::operator::contract::views::PartitionCount(2)
        );
        let first = ClosureParts::get(
            &partitions,
            ryo_execution_domain::domain::operator::contract::views::PartitionIndex(0),
        )?;
        let second = ClosureParts::get(
            &partitions,
            ryo_execution_domain::domain::operator::contract::views::PartitionIndex(1),
        )?;
        assert_eq!(first.key, vec![number(Integer(9))]);
        assert_eq!(first.sources, vec![number(Integer(2)), number(Integer(4))]);
        assert_eq!(
            first.table.cells,
            vec![
                number(Integer(4)),
                number(Integer(3)),
                number(Integer(2)),
                number(Integer(4))
            ]
        );
        assert_eq!(second.key, vec![number(Integer(1))]);
        Ok(())
    }
}

use ryo_values::domain::rows::types::Table;
