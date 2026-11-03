//! Root keys carry orientation and positive reach cannot invent a missing root.

use super::value::DemandAnswers;
use crate::application::file_dag::derived::graph_cache::types::handles::GraphAccess;
use crate::domain::stored_graph::service::build;
use crate::domain::stored_graph::types::{GraphIdentity, GraphWalk, Orientation, Population};
use crate::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{CellKind, Integer, Relation, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

#[test]
fn empty_graph_and_absent_partition_clear_previous_answer() {
    let table = Table {
        kinds: vec![CellKind::Number; 2],
        ..Default::default()
    };
    let graph = build(
        GraphIdentity {
            population: Population {
                relation: Relation::Function,
                kinds: &[CellKind::Number; 2],
                rows: 0,
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
    .expect("empty graph");
    let access = GraphAccess {
        graph: Arc::new(graph),
        orientation: Orientation::Forward,
        shared: false,
    };
    let mut walk = GraphWalk::default();
    walk.reached.push(99);
    let owner = crate::application::file_dag::derived::owner::DerivedOwner::new(
        0,
        crate::outbound::derived::factory::test_port(),
    );
    assert!(
        DemandAnswers::new(&owner, None)
            .reach(&access, &[], Value::Number(Integer(1)), &mut walk)
            .unwrap()
            .is_none()
    );
    assert!(walk.reached.is_empty());
}

use ryo_values::domain::rows::types::Table;
