//! Root keys carry orientation and positive reach cannot invent a missing root.
use super::super::super::constants;
use crate::application::file_dag::derived::owner::fixtures::graph::GraphFixture;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::vertices::types::VertexId;
use ryo_execution_domain::domain::stored_graph::types::GraphWalk;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{
    CellKind, FactColumn, Integer, Relation, Value as CellValue,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput};

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
            PlanInput::Relation(Relation::Function),
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
            .reach(&access, &[], CellValue::Number(Integer(1)), &mut walk)
            .expect(constants::VALID_ROOT_QUERY)
            .is_none()
    );
    assert!(walk.reached.is_empty());
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    EmptyGraphAndAbsentPartitionClearPreviousAnswer,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("empty_graph_and_absent_partition_clear_previous_answer"),
    word: Word::Succeeds,
    input: || Scenario::EmptyGraphAndAbsentPartitionClearPreviousAnswer,
    expect: Expect::Value(()),
}];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::EmptyGraphAndAbsentPartitionClearPreviousAnswer => {
            empty_graph_and_absent_partition_clear_previous_answer();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
