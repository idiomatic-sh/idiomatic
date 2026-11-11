//! Behavioral controls retained from the former application resource constructors.
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_store::application::file_dag::relations::interface::RelationsPort as _;
use ryo_relations::domain::relations::types::{CellKind, Value as Cell};
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
    let owner = ryo_execution_store::application::file_dag::relations::service::RelationOwner::fixture(
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        (std::sync::Arc::new(ryo_execution_store::application::file_dag::fixtures::derived())
            as std::sync::Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort>),
    );
    let RepresentationInput::Computed(table) = input else {
        panic!("computed fixture")
    };
    let publication = owner.publish(ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody::Computed(table.clone()))?;
    let held = owner.rows(&publication, &|_| Err(OperatingError::NotRead))?;
    let input = RepresentationInput::from(&held);
    let proof = owner
        .admit(&publication, input)
        .and_then(|admitted| admitted)?;
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
        ryo_execution_store::application::file_dag::derived::owner::service::ViewContext::default(),
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
) -> Result<crate::application::file_dag::invocation::service::AcquiredPartitions, OperatingError> {
    let owner = std::rc::Rc::new(ryo_execution_store::application::file_dag::relations::service::RelationOwner::fixture(
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        (std::sync::Arc::new(
            ryo_execution_store::application::file_dag::fixtures::derived(),
        ) as std::sync::Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort>),
    ));
    let RepresentationInput::Computed(table) = input else {
        return Err(OperatingError::WrongOperation);
    };
    let publication = owner.publish(ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody::Computed(table.clone()))?;
    let rows = owner.rows(&publication, &|_| Err(OperatingError::NotRead))?;
    let input = RepresentationInput::from(&rows);
    let mut cursor =
        ryo_execution_store::application::file_dag::closure_partitions::service::prepare(
            &publication,
            input,
            keys,
            source,
            destination,
            owner,
        )?;
    let mut parts =
        crate::application::file_dag::invocation::service::AcquiredPartitions::default();
    while cursor.advance()? == ryo_execution_store::application::file_dag::closure_partitions::types::PartitionAdvance::Advanced {
        parts.push(cursor.take_current()?);
    }
    Ok(parts)
}
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::table_words::types::ColumnCount;
fn number(value: Integer) -> Cell {
    Cell::Number(value)
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
    let ryo_derived::domain::contract::types::DerivedView::CycleAdjacency(graph) = graph.view()?
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

use ryo_values::domain::rows::types::Table;

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    AdjacencyKeepsParallelEdgesAndFirstEntityNumbering,
    ClosurePreparationPreservesClusteredPartitionOrderAndSortsSources,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("adjacency_keeps_parallel_edges_and_first_entity_numbering"),
        word: Word::Succeeds,
        input: || Scenario::AdjacencyKeepsParallelEdgesAndFirstEntityNumbering,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("closure_preparation_preserves_clustered_partition_order_and_sorts_sources"),
        word: Word::Succeeds,
        input: || Scenario::ClosurePreparationPreservesClusteredPartitionOrderAndSortsSources,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::AdjacencyKeepsParallelEdgesAndFirstEntityNumbering => {
            adjacency_keeps_parallel_edges_and_first_entity_numbering()
                .map(|_| ())
                .map_err(|error| format!("{error:?}"))
        }
        Scenario::ClosurePreparationPreservesClusteredPartitionOrderAndSortsSources => {
            closure_preparation_preserves_clustered_partition_order_and_sorts_sources()
                .map(|_| ())
                .map_err(|error| format!("{error:?}"))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
