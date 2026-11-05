//! Ordered nearest and forest compression are different from sorted closure order.

use super::value::{ABSENT, GraphTraversal, UNKNOWN};

use crate::domain::stored_graph::types::{GraphDomain, GraphWalk, Orientation};
use ryo_derived::domain::contract::types::{
    BuildOutcome, BuildWorkspace, BulkRows, DerivedFault, DerivedInputs, RowCells, ViewRequest,
};
use ryo_derived::inbound::use_cases::derived::{
    interface::DerivedUseCase, use_case::DerivedUseCaseImpl,
};
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
    fn count(&self) -> usize {
        self.0.count.0
    }
    fn cell(&self, row: usize, column: ColumnIndex) -> Option<Value> {
        if row >= self.count() {
            return None;
        }
        let column = usize::try_from(column.0).ok()?;
        if column >= self.0.kinds.len() {
            return None;
        }
        self.0
            .cells
            .get(row.checked_mul(self.0.kinds.len())?.checked_add(column)?)
            .copied()
    }
}

fn graph(edges: &[[i64; 2]]) -> Arc<ryo_derived::domain::contract::types::CompactGraph> {
    let mut table = Table {
        kinds: vec![CellKind::Number; 2],
        ..Default::default()
    };
    for edge in edges {
        push_row(&mut table, &edge.map(|value| Value::Number(Integer(value))));
    }
    let semantics = DerivedUseCaseImpl::new();
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
}

fn vertex(graph: &ryo_derived::domain::contract::types::CompactGraph, value: i64) -> u32 {
    graph
        .vertex(0, Value::Number(Integer(value)))
        .expect("stored vertex")
}

#[test]
fn bfs_nearest_stops_in_edge_encounter_order_and_direct_retains_duplicates() {
    let graph = graph(&[[1, 9], [1, 2], [1, 9], [9, 4], [2, 4], [4, 1]]);
    assert!(graph.forest(0).is_none());
    let mut domain = GraphDomain {
        identity: None,
        members: Arc::new(vec![false; graph.vertices().len()]),
    };
    Arc::make_mut(&mut domain.members)[vertex(&graph, 9) as usize] = true;
    Arc::make_mut(&mut domain.members)[vertex(&graph, 2) as usize] = true;
    let mut walk = GraphWalk::default();
    let root = vertex(&graph, 1);
    assert!(GraphTraversal::new(&graph, &mut walk).breadth(
        root,
        Orientation::Forward,
        Some(&domain)
    ));
    assert_eq!(walk.reached, [vertex(&graph, 9)]);
    assert_eq!(walk.expanded, 1);
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
    assert_eq!(walk.expanded, 4);
    walk.epoch = u32::MAX;
    let _ = GraphTraversal::new(&graph, &mut walk).breadth(root, Orientation::Forward, None);
    assert_eq!(walk.epoch, 1);
    assert_eq!(walk.reached.len(), 4);
}

#[test]
fn validated_forest_compresses_shared_chains_without_including_the_start() {
    let graph = graph(&[[1, 2], [2, 3], [3, 4], [8, 2], [1, 2]]);
    assert!(graph.forest(0).is_some());
    let mut domain = GraphDomain {
        identity: None,
        members: Arc::new(vec![false; graph.vertices().len()]),
    };
    Arc::make_mut(&mut domain.members)[vertex(&graph, 4) as usize] = true;
    Arc::make_mut(&mut domain.members)[vertex(&graph, 1) as usize] = true;
    let mut walk = GraphWalk::default();
    let mut answers = Vec::new();
    let first = GraphTraversal::new(&graph, &mut walk).nearest(
        vertex(&graph, 1),
        Orientation::Forward,
        &domain,
        &mut answers,
    );
    assert_eq!(first, (Some(vertex(&graph, 4)), false));
    assert_eq!(walk.examined, 3);
    let next = GraphTraversal::new(&graph, &mut walk).nearest(
        vertex(&graph, 8),
        Orientation::Forward,
        &domain,
        &mut answers,
    );
    assert_eq!(next.0, Some(vertex(&graph, 4)));
    assert_eq!(walk.examined, 1);
    assert_eq!(
        GraphTraversal::new(&graph, &mut walk).nearest(
            vertex(&graph, 1),
            Orientation::Forward,
            &domain,
            &mut answers
        ),
        (first.0, true)
    );
    assert_ne!(UNKNOWN, ABSENT);
}

use ryo_values::domain::rows::types::Table;
