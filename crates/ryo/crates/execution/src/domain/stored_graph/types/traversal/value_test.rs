//! Ordered nearest and forest compression are different from sorted closure order.

use super::value::{ABSENT, GraphTraversal, UNKNOWN};

use crate::domain::stored_graph::types::{
    BreadthEnd, ExaminedVertices, ExpandedVertices, GraphDomain, GraphWalk, MemoStatus, Nearest,
    Orientation, VertexId, VisitEpoch,
};
use ryo_derived::domain::contract::types::BuildOutcome;
use ryo_derived::domain::contract::types::BuildWorkspace;
use ryo_derived::domain::row_cells::types::BulkRows;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::contract::types::BuildInputs as DerivedInputs;
use ryo_derived::domain::row_cells::types::RowCells;
use ryo_derived::domain::request::types::DerivedRequest as ViewRequest;
use ryo_derived::domain::sharing::types::SharedMarks;
use ryo_derived::inbound::use_cases::build::interface::BuildUseCase;
use ryo_derived::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_derived::inbound::use_cases::view::interface::ViewUseCase;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::service::push_row;
use std::sync::Arc;

/// The real semantic builder borrows the test's physical source table; this object provides no executor or owner policy.
struct SourceRows<'a>(&'a Table);
impl RowCells for SourceRows<'_> {
    fn bulk(&self) -> Option<BulkRows<'_>> {
        Some(BulkRows::Computed(self.0))
    }
    fn validate(&self) -> Result<(), DerivedFault> {
        if self.0.kinds.len().checked_mul(self.0.count.0) == Some(self.0.cells.len()) {
            Ok(())
        } else {
            Err(DerivedFault::ShapeMismatch)
        }
    }
    fn kinds(&self) -> &[CellKind] {
        &self.0.kinds
    }
    fn count(&self) -> ryo_values::domain::rows::types::RowCount {
        self.0.count
    }
    fn cell(&self, row: ryo_values::domain::rows::types::RowIndex, column: ColumnIndex) -> Option<Value> {
        if row.0 >= self.count().0 {
            return None;
        }
        let column = usize::try_from(column.0).ok()?;
        if column >= self.0.kinds.len() {
            return None;
        }
        self.0
            .cells
            .get(row.0.checked_mul(self.0.kinds.len())?.checked_add(column)?)
            .copied()
    }
}

fn graph(edges: &[[i64; 2]]) -> Arc<ryo_derived::domain::adjacency::types::compact_graph::value::CompactGraph> {
    let mut table = Table {
        kinds: vec![CellKind::Number; 2],
        ..Default::default()
    };
    for edge in edges {
        push_row(&mut table, &edge.map(|value| Value::Number(Integer(value))));
    }
    let semantics = crate::outbound::derived::factory::semantics();
    let prepared = semantics.prepare(
        ViewRequest::Adjacency {
            endpoints: [ColumnIndex(0), ColumnIndex(1)],
            keys: vec![],
        },
        vec![table.kinds.clone()],
    );
    let outcome = semantics
        .build(
            &prepared,
            DerivedInputs::Rows(&SourceRows(&table)),
            &mut BuildWorkspace::default(),
            None,
        )
        .expect("valid graph schema");
    let BuildOutcome::Built(payload) = outcome else {
        panic!("ordinary finite fixture must build its adjacency")
    };
    payload
        .share_adjacency()
        .expect("successful adjacency payload")
        .0
}

fn vertex(graph: &ryo_derived::domain::adjacency::types::compact_graph::value::CompactGraph, value: i64) -> VertexId {
    VertexId(
        graph
            .vertex(ryo_derived::domain::vertices::types::PartitionId(0), Value::Number(Integer(value)))
            .expect("stored vertex")
            .0,
    )
}

#[test]
fn bfs_nearest_stops_in_edge_encounter_order_and_direct_retains_duplicates() {
    let graph = graph(&[[1, 9], [1, 2], [1, 9], [9, 4], [2, 4], [4, 1]]);
    assert!(graph.forest(ryo_derived::domain::vertices::types::EdgeDirection::Forward).is_none());
    let mut marks = vec![false; graph.vertices().len()];
    marks[vertex(&graph, 9).0 as usize] = true;
    marks[vertex(&graph, 2).0 as usize] = true;
    let domain = GraphDomain {
        identity: None,
        members: SharedMarks::from(marks),
    };
    let mut walk = GraphWalk::default();
    let root = vertex(&graph, 1);
    assert_eq!(
        GraphTraversal::new(&graph, &mut walk).breadth(root, Orientation::Forward, Some(&domain)),
        BreadthEnd::Stopped
    );
    assert_eq!(walk.reached, [vertex(&graph, 9)]);
    assert_eq!(walk.expanded, ExpandedVertices(1));
    GraphTraversal::new(&graph, &mut walk).direct(root, Orientation::Forward);
    assert_eq!(
        walk.reached,
        [vertex(&graph, 9), vertex(&graph, 2), vertex(&graph, 9)]
    );
    let _ = GraphTraversal::new(&graph, &mut walk).breadth(root, Orientation::Forward, None);
    assert_eq!(
        walk.reached.iter().filter(|id| **id == root).count(),
        1,
        "positive cycle emits root exactly once"
    );
    assert_eq!(walk.expanded, ExpandedVertices(4));
    walk.epoch = VisitEpoch(u32::MAX);
    let _ = GraphTraversal::new(&graph, &mut walk).breadth(root, Orientation::Forward, None);
    assert_eq!(walk.epoch, VisitEpoch(1));
    assert_eq!(walk.reached.len(), 4);
}

#[test]
fn validated_forest_compresses_shared_chains_without_including_the_start() {
    let graph = graph(&[[1, 2], [2, 3], [3, 4], [8, 2], [1, 2]]);
    assert!(graph.forest(ryo_derived::domain::vertices::types::EdgeDirection::Forward).is_some());
    let mut marks = vec![false; graph.vertices().len()];
    marks[vertex(&graph, 4).0 as usize] = true;
    marks[vertex(&graph, 1).0 as usize] = true;
    let domain = GraphDomain {
        identity: None,
        members: SharedMarks::from(marks),
    };
    let mut walk = GraphWalk::default();
    let mut answers = Vec::new();
    let first = GraphTraversal::new(&graph, &mut walk).nearest(
        vertex(&graph, 1),
        Orientation::Forward,
        &domain,
        &mut answers,
    );
    assert_eq!(
        first,
        Nearest {
            vertex: Some(vertex(&graph, 4)),
            memo: MemoStatus::Fresh
        }
    );
    assert_eq!(walk.examined, ExaminedVertices(3));
    let next = GraphTraversal::new(&graph, &mut walk).nearest(
        vertex(&graph, 8),
        Orientation::Forward,
        &domain,
        &mut answers,
    );
    assert_eq!(next.vertex, Some(vertex(&graph, 4)));
    assert_eq!(walk.examined, ExaminedVertices(1));
    assert_eq!(
        GraphTraversal::new(&graph, &mut walk).nearest(
            vertex(&graph, 1),
            Orientation::Forward,
            &domain,
            &mut answers
        ),
        Nearest {
            vertex: first.vertex,
            memo: MemoStatus::Memoized
        }
    );
    assert_ne!(UNKNOWN, ABSENT);
}

use ryo_values::domain::rows::types::Table;
