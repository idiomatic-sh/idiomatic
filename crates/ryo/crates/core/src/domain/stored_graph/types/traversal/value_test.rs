//! Ordered nearest and forest compression are different from sorted closure order.

use super::value::{ABSENT, GraphTraversal, UNKNOWN};
use crate::domain::stored_graph::service::build;
use crate::domain::stored_graph::types::{
    GraphDomain, GraphIdentity, GraphWalk, Orientation, Population,
};
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{CellKind, Integer, Relation, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

fn graph(edges: &[[i64; 2]]) -> crate::domain::stored_graph::types::CompactGraph {
    let mut table = Table {
        kinds: vec![CellKind::Number; 2],
        ..Default::default()
    };
    for edge in edges {
        push_row(&mut table, &edge.map(|value| Value::Number(Integer(value))));
    }
    build(
        GraphIdentity {
            population: Population {
                relation: Relation::Function,
                kinds: &[CellKind::Number; 2],
                rows: edges.len(),
                native_file: None,
                selected: None,
                positions: None,
            },
            endpoints: [ColumnIndex(0), ColumnIndex(1)],
            keys: vec![],
        },
        &Rows::Computed(&table),
        [ColumnIndex(0), ColumnIndex(1)],
        &[],
    )
    .expect("columns")
    .expect("graph")
}

fn vertex(graph: &crate::domain::stored_graph::types::CompactGraph, value: i64) -> u32 {
    graph.vertex_ids[&(0, Value::Number(Integer(value)))]
}

#[test]
fn bfs_nearest_stops_in_edge_encounter_order_and_direct_retains_duplicates() {
    let graph = graph(&[[1, 9], [1, 2], [1, 9], [9, 4], [2, 4], [4, 1]]);
    assert!(graph.forests[0].is_none());
    let mut domain = GraphDomain {
        identity: None,
        members: Arc::new(vec![false; graph.vertices.len()]),
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
    assert!(graph.forests[0].is_some());
    let mut domain = GraphDomain {
        identity: None,
        members: Arc::new(vec![false; graph.vertices.len()]),
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
