//! The tree relations through `plan` and `run` only: their rows read in place from the open file's elements at the tree
//! nodes the engine names, in tree order, never asked of the facts.
use super::super::super::fixtures::{TableInputs, TextResources, emitted};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::fixtures::node_id;
use ryo_relations::domain::relations::types::{FileId, Position, Relation};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, LiteralText, Operand, PlanComparison,
    QueryLiteral, RepeatedRows, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

#[derive(Clone, Copy)]
enum Scenario {
    /// A select of the `word` tokens, read from every node, from the two `word` nodes, and from none.
    Roots,
    /// A projection of the edges, read from every node.
    Edges,
}
#[derive(Debug, PartialEq)]
enum Observation {
    /// Whether the runs a case compares answered as the case says.
    Same(bool),
    /// How many rows the emitted node holds.
    Count(RowCount),
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_tree_select_reads_its_roots_in_tree_order"),
        word: Word::Succeeds,
        input: || Scenario::Roots,
        expect: Expect::Value(Observation::Same(true)),
    },
    Case {
        name: CaseName("the_edges_of_the_tree_skip_its_root"),
        word: Word::Succeeds,
        input: || Scenario::Edges,
        expect: Expect::Value(Observation::Count(RowCount(3))),
    },
];

/// The rows `step` emits over the small tree, its readers starting from the nodes at positions `at`.
fn from(step: &Step, at: &[u32]) -> Result<Vec<Table>, Failure> {
    let mut resources = TextResources::small_tree();
    let roots = at
        .iter()
        .map(|at| node_id(FileId(0), Position(*at)))
        .collect();
    let inputs = TableInputs::new(&[]).with_roots(roots);
    emitted(
        std::slice::from_ref(step),
        &[PlanNodeId(0)],
        &inputs,
        &mut resources,
        &mut FileBuffers::default(),
    )
}

/// How many rows the first table holds.
fn count(rows: &[Table]) -> RowCount {
    rows.first().map_or(RowCount(0), |table| table.count)
}

fn subject(scenario: Scenario) -> Result<Value, Failure> {
    match scenario {
        Scenario::Roots => {
            let word = PlanComparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(1)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("word".into()))),
                origin: ComparisonOrigin::Unrecorded,
            };
            let step = Step {
                operation: Operation::Select {
                    from: StepInput::Relation(Relation::SyntaxTree),
                    comparisons: vec![word],
                    kept: Vec::new(),
                },
                inputs: vec![PlanInput::Relation(Relation::SyntaxTree)],
            };
            let all = from(&step, &[0, 1, 2, 3])?;
            let named = from(&step, &[1, 3])?;
            let none = from(&step, &[])?;
            Ok(Observation::Same(
                all == named && count(&all) == RowCount(2) && count(&none) == RowCount(0),
            ))
        }
        Scenario::Edges => {
            let step = Step {
                operation: Operation::Project {
                    from: StepInput::Relation(Relation::SyntaxChild),
                    columns: vec![ColumnIndex(1)],
                    distinct: RepeatedRows::Kept,
                },
                inputs: vec![PlanInput::Relation(Relation::SyntaxChild)],
            };
            Ok(Observation::Count(count(&from(&step, &[0, 1, 2, 3])?)))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
