//! Compact adjacency matches the actual public closure owner without enumerating unrelated roots.
use ryo_derived::domain::vertices::types::VertexId;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use super::super::super::fixtures;
use crate::domain::operating::errors::OperatingError;
use ryo_values::domain::rows::errors::RowsError;

use crate::application::file_dag::testing::CompleteTableRows;
use crate::domain::stored_graph::types::traversal::value::GraphTraversal;
use crate::domain::stored_graph::types::{ExaminedVertices, ExpandedVertices, GraphIdentity, GraphWalk, Orientation, RelationPopulation, VisitEpoch};
use crate::domain::table_model::types::Rows;
use ryo_derived::domain::contract::types::BuildOutcome;
use ryo_derived::domain::contract::types::BuildWorkspace;
use ryo_derived::domain::adjacency::types::compact_graph::value::CompactGraph;
use ryo_derived::domain::contract::types::BuildInputs;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Relation, Value as Cell};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput};
use ryo_values::domain::plans::types::Operation;
use std::sync::Arc;
use {ryo_values::domain::rows::service::push_row, ryo_values::domain::rows::service::row_in};

/// Numeric synthetic graph rows, independent of any frontend.
fn table(rows: &[&[i64]], width: usize) -> Table {
    let mut out = Table {
        kinds: vec![CellKind::Number; width],
        ..Table::default()
    };
    for row in rows {
        push_row(
            &mut out,
            &row.iter()
                .map(|value| Cell::Number(Integer(*value)))
                .collect::<Vec<_>>(),
        );
    }
    out
}

/// Identity need not correspond to the schema of the named frontend relation in a standalone graph test.
fn identity(keys: Vec<ColumnIndex>, rows: usize) -> GraphIdentity {
    GraphIdentity {
        population: RelationPopulation {
            relation: Relation::Function,
            kinds: &[CellKind::Number, CellKind::Number, CellKind::Number],
            rows: ryo_values::domain::rows::types::RowCount(rows),
            native_file: None,
        },
        endpoints: [ColumnIndex(0), ColumnIndex(1)],
        keys,
    }
}
fn build(
    identity: GraphIdentity,
    rows: &Rows<'_>,
    endpoints: [ColumnIndex; 2],
    keys: &[ColumnIndex],
) -> Result<Option<Arc<CompactGraph>>, crate::domain::operating::errors::OperatingError> {
    assert_eq!(
        identity.population.rows,
        crate::domain::table_model::service::count_of(rows)
    );
    // The portable request carries the actual endpoints and keys. The old
    // GraphIdentity belongs to the former local builder and is not an input
    // to this port; invalid columns must be refused by the port itself.
    let semantics = (std::sync::Arc::new(
        fixtures::derived(),
    ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>);
    let prepared = semantics.prepare(
        DerivedRequest::Adjacency {
            endpoints,
            keys: keys.to_vec(),
        },
        vec![crate::domain::table_model::service::kinds_of(rows).to_vec()],
    );
    let Rows::Computed(table) = rows else {
        return Err(crate::domain::operating::errors::OperatingError::WrongOperation);
    };
    let complete = CompleteTableRows(table);
    match semantics
        .build(
            &prepared,
            BuildInputs::Rows(&complete),
            &mut BuildWorkspace::default(),
            None,
        )
        .map_err(crate::domain::operator::contract::derived::portable_fault)?
    {
        BuildOutcome::Built(payload) => Ok(payload.share_adjacency().map(|graph| graph.0)),
        BuildOutcome::Declined => Ok(None),
    }
}

fn compact_positive_reach_matches_sorted_closure_with_partitions_cycles_and_duplicates() -> Result<(), Failure> {
    let edges = table(
        &[
            &[9, 2, 1],
            &[2, 3, 1],
            &[9, 2, 1],
            &[3, 9, 1],
            &[2, 2, 1],
            &[9, 8, 2],
            &[8, 7, 2],
            &[40, 41, 3],
        ],
        3,
    );
    let graph = build(
        identity(vec![ColumnIndex(2)], edges.count.0),
        &Rows::Computed(&edges),
        [ColumnIndex(0), ColumnIndex(1)],
        &[ColumnIndex(2)],
    )
    .expect("columns")
    .expect("graph");
    let mut symbols = Symbols::default();
    let (ordinary, _) = fixtures::requests::table_request(
        Operation::Closure {
            edges: PlanInput::Relation(Relation::Function),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
            keys: vec![ColumnIndex(2)],
        },
        &[&edges],
        vec![],
        &mut symbols,
    )
    .expect("ordinary closure through actual owner");
    let mut walk = GraphWalk::default();
    for (root, value) in graph.vertices().iter().enumerate() {
        let partition = graph
            .partition_values(ryo_derived::domain::vertices::types::PartitionId(
                graph.vertex_partitions().0[root],
            ))
            .expect("prepared partition");
        let mut traversal = GraphTraversal::new(&graph, &mut walk);
        let _ = traversal.breadth(VertexId(root as u32), Orientation::Forward, None);
        traversal.sort();
        let actual: Vec<_> = walk
            .reached
            .iter()
            .map(|vertex| graph.vertices()[vertex.0 as usize])
            .collect();
        let expected: Vec<_> = (0..ordinary.count.0)
            .filter_map(|row| row_in(&ordinary, ryo_values::domain::rows::types::RowIndex(row)))
            .filter(|row| row[0] == partition[0] && row[1] == *value)
            .map(|row| row[2])
            .collect();
        assert_eq!(actual, expected, "root {value:?} partition {partition:?}");
    }
    Ok(())
}

fn sparse_demand_work_is_independent_of_unrelated_components() -> Result<(), Failure> {
    let mut edges = table(&[&[1, 2], &[2, 3]], 2);
    for index in 0..300 {
        push_row(
            &mut edges,
            &[
                Cell::Number(Integer(1000 + index * 2)),
                Cell::Number(Integer(1001 + index * 2)),
            ],
        );
    }
    let graph = build(
        identity(vec![], edges.count.0),
        &Rows::Computed(&edges),
        [ColumnIndex(0), ColumnIndex(1)],
        &[],
    )
    .expect("columns")
    .expect("graph");
    let root = graph
        .vertex(ryo_derived::domain::vertices::types::PartitionId(0), Cell::Number(Integer(1)))
        .expect("prepared vertex")
        .0;
    let mut walk = GraphWalk::default();
    let root = VertexId(root);
    let _ = GraphTraversal::new(&graph, &mut walk).breadth(root, Orientation::Forward, None);
    assert_eq!(
        (walk.expanded, walk.examined, walk.reached.len()),
        (ExpandedVertices(3), ExaminedVertices(2), 2)
    );
    // Repeated requests reuse epoch memory, not a global O(vertices) clear.
    let epoch = walk.epoch;
    let _ = GraphTraversal::new(&graph, &mut walk).breadth(root, Orientation::Forward, None);
    assert_eq!(walk.epoch, VisitEpoch(epoch.0 + 1));
    assert_eq!(
        (walk.expanded, walk.examined),
        (ExpandedVertices(3), ExaminedVertices(2))
    );
    Ok(())
}

fn invalid_columns_are_rejected_even_on_empty_edges() -> Result<(), Failure> {
    let edges = table(&[], 2);
    assert!(matches!(
        build(
            identity(vec![], 0),
            &Rows::Computed(&edges),
            [ColumnIndex(0), ColumnIndex(2)],
            &[]
        ),
        Err(OperatingError::Table(RowsError::NoSuchColumn { .. }))
    ));
    assert!(matches!(
        build(
            identity(vec![ColumnIndex(3)], 0),
            &Rows::Computed(&edges),
            [ColumnIndex(0), ColumnIndex(1)],
            &[ColumnIndex(3)]
        ),
        Err(OperatingError::Table(RowsError::NoSuchColumn { .. }))
    ));
    Ok(())
}

fn typed_endpoint_identity_never_coalesces_equal_payload_words() -> Result<(), Failure> {
    let mut edges = Table {
        kinds: vec![CellKind::Number; 2],
        ..Default::default()
    };
    let number = Cell::Number(Integer(0));
    let flag = Cell::Flag(ryo_relations::domain::relations::types::Flag::Lowered);
    push_row(&mut edges, &[number, flag]);
    push_row(&mut edges, &[flag, number]);
    let graph = build(
        identity(vec![], 2),
        &Rows::Computed(&edges),
        [ColumnIndex(0), ColumnIndex(1)],
        &[],
    )
    .expect("columns")
    .expect("graph");
    assert_eq!(graph.vertices().len(), 2);
    assert_ne!(graph.vertex(ryo_derived::domain::vertices::types::PartitionId(0), number), graph.vertex(ryo_derived::domain::vertices::types::PartitionId(0), flag));
    Ok(())
}

use ryo_values::domain::rows::types::Table;
/// The scenarios of this group, one row each.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    CompactPositiveReachMatchesSortedClosureWithPartitionsCyclesAndDuplicates,
    SparseDemandWorkIsIndependentOfUnrelatedComponents,
    InvalidColumnsAreRejectedEvenOnEmptyEdges,
    TypedEndpointIdentityNeverCoalescesEqualPayloadWords,
}
type Input = Scenario;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(compact_positive_reach_matches_sorted_closure_with_partitions_cycles_and_duplicates)),
        word: Word::Succeeds,
        input: || Scenario::CompactPositiveReachMatchesSortedClosureWithPartitionsCyclesAndDuplicates,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(sparse_demand_work_is_independent_of_unrelated_components)),
        word: Word::Succeeds,
        input: || Scenario::SparseDemandWorkIsIndependentOfUnrelatedComponents,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(invalid_columns_are_rejected_even_on_empty_edges)),
        word: Word::Succeeds,
        input: || Scenario::InvalidColumnsAreRejectedEvenOnEmptyEdges,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(typed_endpoint_identity_never_coalesces_equal_payload_words)),
        word: Word::Succeeds,
        input: || Scenario::TypedEndpointIdentityNeverCoalescesEqualPayloadWords,
        expect: Expect::Value(()),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::CompactPositiveReachMatchesSortedClosureWithPartitionsCyclesAndDuplicates => compact_positive_reach_matches_sorted_closure_with_partitions_cycles_and_duplicates(),
        Scenario::SparseDemandWorkIsIndependentOfUnrelatedComponents => sparse_demand_work_is_independent_of_unrelated_components(),
        Scenario::InvalidColumnsAreRejectedEvenOnEmptyEdges => invalid_columns_are_rejected_even_on_empty_edges(),
        Scenario::TypedEndpointIdentityNeverCoalescesEqualPayloadWords => typed_endpoint_identity_never_coalesces_equal_payload_words(),
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
