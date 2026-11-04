//! Root keys carry orientation and positive reach cannot invent a missing root.
use crate::application::file_dag::derived::owner::graph_fixture::GraphFixture;
use crate::domain::stored_graph::types::GraphWalk;
use crate::domain::table_model::types::Rows;
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
        crate::application::file_dag::derived::indexes::LIMIT,
    );
    let access = fixture
        .acquire(
            Input::Relation(Relation::Function),
            &rows,
            [ColumnIndex(0), ColumnIndex(1)],
            &[],
        )
        .expect("valid columns")
        .expect("empty graph");
    let mut walk = GraphWalk::default();
    walk.reached.push(99);
    assert!(
        fixture
            .reach(&access, &[], Value::Number(Integer(1)), &mut walk)
            .expect("valid root query")
            .is_none()
    );
    assert!(walk.reached.is_empty());
}
