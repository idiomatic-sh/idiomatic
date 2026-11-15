//! The tree relations through `plan` and `run` only: their rows read in place from the open file's elements at the tree
//! nodes the engine names, in tree order, never asked of the facts.
use super::super::super::fixtures::cases::defines_of_nodes;
use super::super::super::fixtures::{TableInputs, TextResources, emitted, planned};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, PlanInput, PlanInputList, PlanNodeId, ProbeIndex,
};
use ryo_relations::domain::relations::fixtures::node_id;
use ryo_relations::domain::relations::types::{FileId, Position, Relation};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, JoinColumn, LiteralText, Operand,
    PlanComparison, QueryLiteral, RepeatedRows, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

#[derive(Clone, Copy)]
enum Scenario {
    /// A select of the `word` tokens, read from every node, from the two `word` nodes, and from none.
    Roots,
    /// A projection of the edges, read from every node.
    Edges,
    /// A join of two nodes, a node of three children and a token, with the edges of the nested tree on their parent.
    ProbeFed,
    /// The same join, its index builds counted.
    ProbeBuilds,
    /// An antijoin of rows narrower than their key with the edges of a tree of only its root.
    AntijoinOfNoEdges,
}
#[derive(Debug, PartialEq)]
enum Observation {
    /// Whether the runs a case compares answered as the case says.
    Same(bool),
    /// How many rows the emitted node holds.
    Count(RowCount),
    /// The rows the emitted node was fed from each input.
    Fed([usize; 3]),
    /// How many probe indexes the run built.
    Builds(usize),
    /// How many rows the emitted node holds, and whether its probe walks the tree.
    Kept(RowCount, bool),
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
    Case {
        name: CaseName("a_probe_of_the_tree_is_fed_the_rows_it_finds"),
        word: Word::Succeeds,
        input: || Scenario::ProbeFed,
        // Two left rows, and the node's three children of the tree's five edges: the token has none.
        expect: Expect::Value(Observation::Fed([2, 3, 0])),
    },
    Case {
        name: CaseName("a_probe_of_the_tree_builds_no_index"),
        word: Word::Succeeds,
        input: || Scenario::ProbeBuilds,
        expect: Expect::Value(Observation::Builds(0)),
    },
    Case {
        name: CaseName("an_antijoin_of_no_edges_keeps_its_rows_unread"),
        word: Word::Succeeds,
        input: || Scenario::AntijoinOfNoEdges,
        expect: Expect::Value(Observation::Kept(RowCount(2), true)),
    },
];

/// The join of `DEFINES`' node column with the edges on their parent, answering the left node and the child.
fn children_of_defines() -> Step {
    Step {
        operation: Operation::Join {
            left: StepInput::Relation(Relation::Defines),
            right: StepInput::Relation(Relation::SyntaxChild),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(1),
                },
            ],
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        },
        inputs: vec![
            PlanInput::Relation(Relation::Defines),
            PlanInput::Relation(Relation::SyntaxChild),
        ],
    }
}

/// The buffers of the join of `DEFINES` rows of the nested tree's `other` node and its first token with the tree's edges,
/// read from every node: the node has three children, the token none, and the tree five edges.
fn probed_buffers() -> Result<FileBuffers, Failure> {
    let mut resources = TextResources::nested_tree();
    let rows = defines_of_nodes(
        &[
            node_id(FileId(0), Position(1)),
            node_id(FileId(0), Position(2)),
        ],
        resources.symbol("word"),
    );
    let inputs = TableInputs::new(&[(Relation::Defines, &rows)])
        .with_roots((0..6).map(|at| node_id(FileId(0), Position(at))).collect());
    let mut buffers = FileBuffers::default();
    let _rows = emitted(
        &[children_of_defines()],
        &[PlanNodeId(0)],
        &inputs,
        &mut resources,
        &mut buffers,
    )?;
    Ok(buffers)
}

/// The antijoin of `DEFINES`' node column, projected alone, keyed on its sixth column, with the edges of a tree of only its
/// root: how many rows it keeps, and whether its plan walks the tree.
fn antijoin_of_no_edges() -> Result<Value, Failure> {
    let mut resources = TextResources::root_only();
    let root = node_id(FileId(0), Position(0));
    let rows = defines_of_nodes(&[root, root], resources.symbol("file"));
    let inputs = TableInputs::new(&[(Relation::Defines, &rows)])
        .with_roots(vec![node_id(FileId(0), Position(0))]);
    let steps = [
        Step {
            operation: Operation::Project {
                from: StepInput::Relation(Relation::Defines),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
            inputs: vec![PlanInput::Relation(Relation::Defines)],
        },
        Step {
            operation: Operation::Antijoin {
                left: StepInput::Step(StepId(0)),
                right: StepInput::Relation(Relation::SyntaxChild),
                left_on: vec![ColumnIndex(5)],
                right_on: vec![ColumnIndex(0)],
            },
            inputs: vec![
                PlanInput::Node(PlanNodeId(0)),
                PlanInput::Relation(Relation::SyntaxChild),
            ],
        },
    ];
    let mut program = LogicalProgram::new();
    for step in &steps {
        let _id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    program.emit(RuleOrdinal(0), PlanNodeId(1));
    let plan = planned(&program)?;
    let walks = plan.nodes.get(1).is_some_and(|node| {
        matches!(
            node.lowering,
            Lowering::Probe {
                index: ProbeIndex::Tree,
                ..
            }
        )
    });
    let rows = emitted(
        &steps,
        &[PlanNodeId(1)],
        &inputs,
        &mut resources,
        &mut FileBuffers::default(),
    )?;
    Ok(Observation::Kept(count(&rows), walks))
}

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
        Scenario::ProbeFed => Ok(Observation::Fed(
            probed_buffers()?.fed(PlanNodeId(0)).0.map(|rows| rows.0),
        )),
        Scenario::ProbeBuilds => Ok(Observation::Builds(probed_buffers()?.builds.0)),
        Scenario::AntijoinOfNoEdges => antijoin_of_no_edges(),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
