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

/// The edge columns the graph is acquired over, a vertex an earlier walk left reached, and the
/// root reach starts from.
#[derive(Debug)]
struct Reach {
    edges: [&'static [u64]; 2],
    previous: u32,
    root: i64,
}

/// Whether reach answered a root, and how many vertices the walk holds after it.
#[derive(Debug, PartialEq)]
struct Reached {
    answered: bool,
    reached: usize,
}

type Input = Reach;
type Value = Reached;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("empty_graph_and_absent_partition_clear_previous_answer"),
    word: Word::Succeeds,
    input: || Reach {
        edges: [&[], &[]],
        previous: 99,
        root: 1,
    },
    expect: Expect::Value(Reached {
        answered: false,
        reached: 0,
    }),
}];

/// Acquires the graph over the edges, leaves the previous vertex in the walk, and reaches from
/// the root with no partition.
fn subject(input: Input) -> Result<Value, Failure> {
    let columns = input.edges.map(|column| FactColumn(column.to_vec()));
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
    walk.reached.push(VertexId(input.previous));
    let answered = fixture
        .reach(
            &access,
            &[],
            CellValue::Number(Integer(input.root)),
            &mut walk,
        )
        .expect(constants::VALID_ROOT_QUERY)
        .is_some();
    Ok(Reached {
        answered,
        reached: walk.reached.len(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
