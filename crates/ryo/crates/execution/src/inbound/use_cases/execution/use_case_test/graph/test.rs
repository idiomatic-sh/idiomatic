//! Compact adjacency matches the actual public closure owner without enumerating unrelated roots.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_derived::domain::vertices::types::VertexId;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rows::errors::RowsError;

use ryo_derived::domain::adjacency::types::compact_graph::value::CompactGraph;
use ryo_derived::domain::contract::types::BuildInputs;
use ryo_derived::domain::contract::types::BuildOutcome;
use ryo_derived::domain::contract::types::BuildWorkspace;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_execution_domain::domain::stored_graph::types::traversal::value::GraphTraversal;
use ryo_execution_domain::domain::stored_graph::types::{
    GraphIdentity, GraphWalk, Orientation, RelationPopulation,
};
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_runtime::application::file_dag::testing::types::CompleteTableRows;
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
) -> Result<Option<Arc<CompactGraph>>, OperatingError> {
    if identity.population.rows
        != ryo_execution_domain::domain::table_model::service::count_of(rows)
    {
        return Err(OperatingError::ShapeMismatch);
    }
    // The portable request carries the actual endpoints and keys. The old
    // GraphIdentity belongs to the former local builder and is not an input
    // to this port; invalid columns must be refused by the port itself.
    let semantics: Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort> =
        Arc::new(fixtures::derived());
    let prepared = semantics.prepare(
        DerivedRequest::Adjacency {
            endpoints,
            keys: keys.to_vec(),
        },
        vec![ryo_execution_domain::domain::table_model::service::kinds_of(rows).to_vec()].into(),
    );
    let Rows::Computed(table) = rows else {
        return Err(OperatingError::WrongOperation);
    };
    let complete = CompleteTableRows(table);
    match semantics
        .build(
            &prepared,
            BuildInputs::Rows(&complete),
            &mut BuildWorkspace::default(),
            None.into(),
        )
        .map_err(ryo_execution_domain::domain::operating::service::derived_fault)?
    {
        BuildOutcome::Built(payload) => Ok(payload.share_adjacency().map(|graph| graph.0)),
        BuildOutcome::Declined => Ok(None),
    }
}

/// Builds the keyed graph of partitioned edges with cycles and duplicates, and walks forward from
/// each vertex, comparing what it reaches with the ordinary closure's rows for that root.
fn reach_against_closure() -> Result<Value, Failure> {
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
    )?
    .ok_or(OperatingError::NotRead)?;
    let mut symbols = Symbols::default();
    let ordinary = fixtures::requests::table_request(
        Operation::Closure {
            edges: PlanInput::Relation(Relation::Function),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
            keys: vec![ColumnIndex(2)],
        },
        &[&edges],
        vec![],
        &mut symbols,
    )?;
    let mut walk = GraphWalk::default();
    let mut mismatched = 0;
    for (root, value) in graph.vertices().iter().enumerate() {
        let partition = graph
            .partition_values(ryo_derived::domain::vertices::types::PartitionId(
                graph.vertex_partitions().0[root],
            ))
            .ok_or(OperatingError::NotRead)?;
        let mut traversal = GraphTraversal::new(&graph, &mut walk);
        let _ = traversal.breadth(
            VertexId(u32::try_from(root).expect("a vertex number fits in u32")),
            Orientation::Forward,
            None,
        );
        traversal.sort();
        let actual: Vec<_> = walk
            .reached
            .iter()
            .map(|vertex| {
                graph.vertices()[usize::try_from(vertex.0).expect("a vertex number fits in usize")]
            })
            .collect();
        let expected: Vec<_> = (0..ordinary.count.0)
            .filter_map(|row| row_in(&ordinary, ryo_values::domain::rows::types::RowIndex(row)))
            .filter(|row| row[0] == partition[0] && row[1] == *value)
            .map(|row| row[2])
            .collect();
        mismatched += usize::from(actual != expected);
    }
    Ok(Graphed::Reach {
        roots: graph.vertices().len(),
        mismatched,
    })
}

/// Builds a graph of the path 1, 2, 3 beside 300 unrelated edges, and walks forward from 1 twice.
fn sparse_walk() -> Result<Value, Failure> {
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
    )?
    .ok_or(OperatingError::NotRead)?;
    let root = graph
        .vertex(
            ryo_derived::domain::vertices::types::PartitionId(0),
            Cell::Number(Integer(1)),
        )
        .ok_or(OperatingError::NotRead)?
        .0;
    let mut walk = GraphWalk::default();
    let root = VertexId(root);
    let _ = GraphTraversal::new(&graph, &mut walk).breadth(root, Orientation::Forward, None);
    let first = (walk.expanded.0, walk.examined.0, walk.reached.len());
    let epoch = walk.epoch;
    let _ = GraphTraversal::new(&graph, &mut walk).breadth(root, Orientation::Forward, None);
    Ok(Graphed::Walked {
        first,
        again: (walk.expanded.0, walk.examined.0),
        epochs: walk.epoch.0 - epoch.0,
    })
}

/// Builds a graph over empty two-column edges with the endpoints and keys.
fn empty_edges(endpoints: [u32; 2], keys: &[u32]) -> Result<Value, Failure> {
    let edges = table(&[], 2);
    let keys: Vec<ColumnIndex> = keys.iter().copied().map(ColumnIndex).collect();
    let graph = build(
        identity(keys.clone(), 0),
        &Rows::Computed(&edges),
        endpoints.map(ColumnIndex),
        &keys,
    )?;
    Ok(Graphed::Built(graph.map(|graph| graph.vertices().len())))
}

/// Builds a graph of a number and a flag whose payload words are equal, each an endpoint of the other.
fn typed_endpoints() -> Result<Value, Failure> {
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
    )?
    .ok_or(OperatingError::NotRead)?;
    let partition = ryo_derived::domain::vertices::types::PartitionId(0);
    Ok(Graphed::Vertices {
        count: graph.vertices().len(),
        distinct: graph.vertex(partition, number) != graph.vertex(partition, flag),
    })
}

use ryo_values::domain::rows::types::Table;
/// The graph a row builds.
#[derive(Clone, Copy, Debug)]
enum Graph {
    /// Keyed edges with partitions, cycles and duplicates, walked from every vertex.
    PartitionedCycles,
    /// A short path beside many unrelated components, walked from its start twice.
    SparseComponents,
    /// Empty two-column edges with these endpoint and key columns.
    EmptyEdges {
        endpoints: [u32; 2],
        keys: &'static [u32],
    },
    /// Two typed endpoints with equal payload words.
    TypedEndpoints,
}

/// What building and walking the graph shows.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Graphed {
    /// The roots walked, and those whose reach is not the ordinary closure's.
    Reach { roots: usize, mismatched: usize },
    /// The expanded, examined and reached vertices of the first walk, the expanded and examined of
    /// the second, and the visit epochs between them.
    Walked {
        first: (u64, u64, usize),
        again: (u64, u64),
        epochs: u32,
    },
    /// The vertices of the graph built, or none when the port declined to build one.
    Built(Option<usize>),
    /// The vertices, and whether the two endpoints are distinct vertices.
    Vertices { count: usize, distinct: bool },
}

type Input = Graph;
type Value = Graphed;
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(
            compact_positive_reach_matches_sorted_closure_with_partitions_cycles_and_duplicates
        )),
        word: Word::Succeeds,
        input: || Graph::PartitionedCycles,
        expect: Expect::Value(Graphed::Reach {
            roots: 8,
            mismatched: 0,
        }),
    },
    Case {
        // Repeated requests reuse epoch memory, not a global clear of every vertex.
        name: CaseName(stringify!(
            sparse_demand_work_is_independent_of_unrelated_components
        )),
        word: Word::Succeeds,
        input: || Graph::SparseComponents,
        expect: Expect::Value(Graphed::Walked {
            first: (3, 2, 2),
            again: (3, 2),
            epochs: 1,
        }),
    },
    Case {
        name: CaseName("empty_edges_with_valid_columns_build_no_vertex"),
        word: Word::Succeeds,
        input: || Graph::EmptyEdges {
            endpoints: [0, 1],
            keys: &[],
        },
        expect: Expect::Value(Graphed::Built(Some(0))),
    },
    Case {
        name: CaseName(stringify!(invalid_columns_are_rejected_even_on_empty_edges)),
        word: Word::Refuses,
        input: || Graph::EmptyEdges {
            endpoints: [0, 2],
            keys: &[],
        },
        expect: Expect::Error(OperatingError::Table(RowsError::NoSuchColumn { index: 2 })),
    },
    Case {
        name: CaseName("an_invalid_key_column_is_rejected_on_empty_edges"),
        word: Word::Refuses,
        input: || Graph::EmptyEdges {
            endpoints: [0, 1],
            keys: &[3],
        },
        expect: Expect::Error(OperatingError::Table(RowsError::NoSuchColumn { index: 3 })),
    },
    Case {
        name: CaseName(stringify!(
            typed_endpoint_identity_never_coalesces_equal_payload_words
        )),
        word: Word::Succeeds,
        input: || Graph::TypedEndpoints,
        expect: Expect::Value(Graphed::Vertices {
            count: 2,
            distinct: true,
        }),
    },
];
/// Builds and observes the graph.
fn subject(graph: Input) -> Result<Value, Failure> {
    match graph {
        Graph::PartitionedCycles => reach_against_closure(),
        Graph::SparseComponents => sparse_walk(),
        Graph::EmptyEdges { endpoints, keys } => empty_edges(endpoints, keys),
        Graph::TypedEndpoints => typed_endpoints(),
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
