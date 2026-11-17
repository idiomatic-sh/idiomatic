//! The axis kind through `plan` and `run` only: one node read as two or three of its inputs answers as separate nodes do,
//! and the answer's layout and column checks are main's whatever the start rows, the reached column a node.
use super::super::super::fixtures::{TableInputs, TextResources, emitted};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Integer, Position, Relation, Value as Cell,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Direction, Input as StepInput, JoinColumn, Side, StepId,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// The relation the cases read edges from: parent, child, ordinal, field, file.
const EDGES: Relation = Relation::SemanticChild;

/// What a case runs.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The starts, edges and domain, by the select node each reads (0, 1 or 2), against three nodes of their own.
    Shared([u32; 3]),
    /// An axis over no start rows: its start column, and its declared output, none for the start row then the node.
    NoStarts(u32, Option<[JoinColumn; 2]>),
    /// An axis over edges whose child column is a symbol, from one start: the kinds of its answer.
    SymbolChild,
}
/// What a case observed.
#[derive(Debug, PartialEq, Eq)]
enum Observation {
    /// Whether the shared and the separate inputs answered the same rows, and some.
    Same(bool),
    /// The kinds of the answer, and how many rows it holds.
    Layout(std::borrow::Cow<'static, [CellKind]>, usize),
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

/// The kinds of `semantic-child` with a node after them.
const EDGES_AND_NODE: &[CellKind] = &[
    CellKind::Node,
    CellKind::Node,
    CellKind::Number,
    CellKind::Symbol,
    CellKind::File,
    CellKind::Node,
];

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected
/// value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_axis_whose_domain_is_its_starts_answers_as_over_separate_nodes"),
        word: Word::Succeeds,
        input: || Scenario::Shared([0, 1, 0]),
        expect: Expect::Value(Observation::Same(true)),
    },
    Case {
        name: CaseName("an_axis_whose_domain_is_its_edges_answers_as_over_separate_nodes"),
        word: Word::Succeeds,
        input: || Scenario::Shared([0, 1, 1]),
        expect: Expect::Value(Observation::Same(true)),
    },
    Case {
        name: CaseName("an_axis_over_one_node_for_all_three_inputs_answers_as_over_separate_nodes"),
        word: Word::Succeeds,
        input: || Scenario::Shared([0, 0, 0]),
        expect: Expect::Value(Observation::Same(true)),
    },
    Case {
        name: CaseName("an_axis_over_no_start_rows_answers_the_start_row_then_a_node"),
        word: Word::Succeeds,
        input: || Scenario::NoStarts(0, None),
        expect: Expect::Value(Observation::Layout(
            std::borrow::Cow::Borrowed(EDGES_AND_NODE),
            0,
        )),
    },
    Case {
        name: CaseName("an_axis_over_no_start_rows_reads_its_right_output_kinds_from_the_domain"),
        word: Word::Succeeds,
        input: || {
            Scenario::NoStarts(
                0,
                Some([
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(3),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(2),
                    },
                ]),
            )
        },
        expect: Expect::Value(Observation::Layout(
            std::borrow::Cow::Borrowed(&[CellKind::Symbol, CellKind::Number]),
            0,
        )),
    },
    Case {
        name: CaseName("an_axis_over_no_start_rows_refuses_a_start_column_past_its_rows"),
        word: Word::Refuses,
        input: || Scenario::NoStarts(99, None),
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn {
            index: ColumnIndex(99),
        }),
    },
    Case {
        name: CaseName("an_axis_over_no_start_rows_refuses_an_output_column_past_the_domain"),
        word: Word::Refuses,
        input: || {
            Scenario::NoStarts(
                0,
                Some([
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(0),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(98),
                    },
                ]),
            )
        },
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn {
            index: ColumnIndex(98),
        }),
    },
    Case {
        name: CaseName("an_axis_declares_its_reached_column_a_node"),
        word: Word::Succeeds,
        input: || Scenario::SymbolChild,
        expect: Expect::Value(Observation::Layout(
            std::borrow::Cow::Borrowed(EDGES_AND_NODE),
            0,
        )),
    },
];

/// The node at position `at` of file 0.
fn node(at: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(at)))
}

/// Rows of `EDGES`, parent to child, with the field `f`.
fn edges(pairs: &[(u32, u32)], texts: &TextResources) -> Table {
    let mut cells = Vec::with_capacity(pairs.len() * kinds(EDGES).len());
    for (parent, child) in pairs {
        cells.extend([
            node(*parent),
            node(*child),
            Cell::Number(Integer(0)),
            texts.symbol("f"),
            Cell::File(FileId(0)),
        ]);
    }
    Table {
        kinds: kinds(EDGES).to_vec(),
        cells,
        count: RowCount(pairs.len()),
    }
}

/// Every row of `EDGES`.
fn every_edge() -> Step {
    Step {
        operation: Operation::Select {
            from: StepInput::Relation(EDGES),
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
        inputs: vec![PlanInput::Relation(EDGES)],
    }
}

/// The rows of `EDGES` whose field is `field`: every row for `f`, none for any other text.
fn starts(field: &str) -> Step {
    use ryo_values::domain::plan_parts::types::{
        ComparisonOrigin, LiteralText, Operand, PlanComparison, QueryLiteral, Test,
    };
    Step {
        operation: Operation::Select {
            from: StepInput::Relation(EDGES),
            comparisons: vec![PlanComparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(3)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText(field.into()))),
                origin: ComparisonOrigin::Unrecorded,
            }],
            kept: Vec::new(),
        },
        inputs: vec![PlanInput::Relation(EDGES)],
    }
}

/// An axis from the children of its starts' parents, the domain's child admitted.
fn axis(inputs: [PlanInput; 3], id: u32, child: u32, output: Vec<JoinColumn>) -> Step {
    let input_of = |input: PlanInput| match input {
        PlanInput::Node(PlanNodeId(at)) => StepInput::Step(StepId(at)),
        PlanInput::Relation(relation) => StepInput::Relation(relation),
        PlanInput::Frontier(_) | PlanInput::Observation(_) => StepInput::Relation(EDGES),
    };
    let [from, edges, domain] = inputs;
    Step {
        operation: Operation::Axis {
            from: input_of(from),
            edges: input_of(edges),
            domain: input_of(domain),
            direction: Direction::Child,
            id: ColumnIndex(id),
            parent: ColumnIndex(0),
            child: ColumnIndex(child),
            domain_id: ColumnIndex(1),
            domain_filter: Vec::new(),
            domain_view: Vec::new(),
            output,
        },
        inputs: inputs.to_vec(),
    }
}

/// The rows the last step answers over `tables`.
fn answered(
    steps: &[Step],
    tables: &[(Relation, &Table)],
    resources: &mut TextResources,
) -> Result<Table, Failure> {
    let inputs = TableInputs::new(tables);
    let last = PlanNodeId(u32::try_from(steps.len() - 1).unwrap_or(u32::MAX));
    let rows = emitted(
        steps,
        &[last],
        &inputs,
        resources,
        &mut FileBuffers::default(),
    )?;
    rows.into_iter().next().ok_or(PortableOperatorFault::NotRun)
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut resources = TextResources::new(&["f"]);
    let graph = edges(&[(1, 2), (1, 3), (2, 4), (3, 5)], &resources);
    match scenario {
        Scenario::Shared(reads) => {
            let node_input = |at: u32| PlanInput::Node(PlanNodeId(at));
            let shared = [
                every_edge(),
                every_edge(),
                axis(reads.map(node_input), 0, 1, Vec::new()),
            ];
            let separate = [
                every_edge(),
                every_edge(),
                every_edge(),
                axis([0, 1, 2].map(node_input), 0, 1, Vec::new()),
            ];
            let one = answered(&shared, &[(EDGES, &graph)], &mut resources)?;
            let other = answered(&separate, &[(EDGES, &graph)], &mut resources)?;
            Ok(Observation::Same(one == other && one.count.0 > 0))
        }
        Scenario::NoStarts(id, output) => {
            let walk = axis(
                [
                    PlanInput::Node(PlanNodeId(0)),
                    PlanInput::Relation(EDGES),
                    PlanInput::Relation(EDGES),
                ],
                id,
                1,
                output.map_or_else(Vec::new, |columns| columns.to_vec()),
            );
            let table = answered(
                &[starts("absent"), walk],
                &[(EDGES, &graph)],
                &mut resources,
            )?;
            Ok(Observation::Layout(
                std::borrow::Cow::Owned(table.kinds),
                table.count.0,
            ))
        }
        Scenario::SymbolChild => {
            let walk = axis(
                [
                    PlanInput::Node(PlanNodeId(0)),
                    PlanInput::Relation(EDGES),
                    PlanInput::Relation(EDGES),
                ],
                0,
                3,
                Vec::new(),
            );
            let table = answered(&[starts("f"), walk], &[(EDGES, &graph)], &mut resources)?;
            Ok(Observation::Layout(
                std::borrow::Cow::Owned(table.kinds),
                table.count.0,
            ))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
