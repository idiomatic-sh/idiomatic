//! Root keys carry orientation and positive reach cannot invent a missing root.
use ryo_derived::domain::vertices::types::VertexId;
use super::constants;
use crate::application::file_dag::derived::owner::fixtures::graph::GraphFixture;
use ryo_execution_domain::domain::stored_graph::types::GraphWalk;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{CellKind, FactColumn, Integer, Relation, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};

#[test]
fn empty_graph_and_absent_partition_clear_previous_answer() {
    let columns = [FactColumn(vec![]), FactColumn(vec![])];
    let rows = Rows::Stored(ryo_relations::domain::relations::types::ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    });
    let fixture = GraphFixture::new(
        &[Relation::Function],
        &[],
        crate::application::file_dag::derived::owner::index::service::LIMIT,
    );
    let access = fixture
        .acquire(
            Input::Relation(Relation::Function),
            &rows,
            [ColumnIndex(0), ColumnIndex(1)],
            &[],
        )
        .expect(constants::VALID_COLUMNS)
        .expect(constants::EMPTY_GRAPH);
    let mut walk = GraphWalk::default();
    walk.reached.push(VertexId(99));
    assert!(
        fixture
            .reach(&access, &[], Value::Number(Integer(1)), &mut walk)
            .expect(constants::VALID_ROOT_QUERY)
            .is_none()
    );
    assert!(walk.reached.is_empty());
}
