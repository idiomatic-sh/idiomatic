//! What every kind keeps across kinds, through `plan` and `run` only: which kinds answer a row once and which keep its
//! repeats, the order a select, a project and a join answer in, an antijoin over no right rows, a counted grouping's
//! distinct count, the same rows from a relation's columns and from an earlier node's table, and that a program reads only
//! the inputs its steps name.
use super::super::super::fixtures::cases::{DEFINES, defines, input_of, named, select};
use super::super::super::fixtures::{TableInputs, TextResources, emitted, planned, ran};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{Step, text_program};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::{
    Candidates, FactRows, LogicalNode, NodeBindings, PlanInput, PlanInputList, PlanInputs,
    PlanNodeId, RuleOrdinal,
};
use ryo_relations::domain::relations::fixtures::node_id;
use ryo_relations::domain::relations::types::{FileId, Integer, Position, Relation, Value as Cell};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, JoinColumn, ObservationBindingId, Operand, ParameterId, QueryLiteral,
    RepeatedRows, Side, StepScope,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;

/// The texts of the file the cases read: the refinements and names of their `DEFINES` rows.
const TEXTS: &[&str] = &["kind", "r1", "r2", "a", "b", "zzz"];

#[derive(Clone, Copy, Debug)]
enum Scenario {
    SetOperations,
    AntijoinWithNoRightRows,
    JoinOrder,
    FactsAndTable,
    GroupCount,
    ReadsOnlyItsInputs,
    SelectOrder,
    ProjectZeroColumns,
    AntijoinOfPresentAndAbsentKeys,
    AntijoinOfNoRightRowsPastItsKey,
    AntijoinOfRightRowsPastItsKey,
    JoinThroughAView,
    JoinOfNoMatchPastItsOutput,
    JoinPastItsLeftKey,
}
type Input = Scenario;
/// The rows each emitted node answers, one node per `;`, each cell a node's place, a text or a number.
type Value = Cow<'static, str>;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("set_operations_keep_a_row_once"),
        word: Word::Succeeds,
        input: || Scenario::SetOperations,
        // A projection keeping repeats, then the union, the intersection and the difference of it, a projection dropping
        // repeats, and the extension of the first: the set operations and the dropping projection answer a row once, the
        // others keep its repeats.
        expect: Expect::Value(Cow::Borrowed(
            "a | b | a; a | b; a | b; a; a | b; a 1 | b 1 | a 1",
        )),
    },
    Case {
        name: CaseName("antijoin_with_no_right_rows_is_left"),
        word: Word::Succeeds,
        input: || Scenario::AntijoinWithNoRightRows,
        expect: Expect::Value(Cow::Borrowed("a | b | a")),
    },
    Case {
        name: CaseName("join_keeps_left_order_then_right_order"),
        word: Word::Succeeds,
        input: || Scenario::JoinOrder,
        // Left rows b, a, b joined on the name with the same rows: each left row in order, its right rows in theirs.
        expect: Expect::Value(Cow::Borrowed("n0 n0 | n0 n2 | n1 n1 | n2 n0 | n2 n2")),
    },
    Case {
        name: CaseName("facts_and_table_inputs_answer_the_same"),
        word: Word::Succeeds,
        input: || Scenario::FactsAndTable,
        // A select, a projection, a counted grouping and a join, each over the relation's columns and over a node holding
        // the same rows as a table: one line each, the two answers the same.
        expect: Expect::Value(Cow::Borrowed(
            "n1 a; n0 b | n1 a | n2 b; r1 2 | r2 1; n0 n0 | n0 n2 | n1 n1 | n2 n0 | n2 n2",
        )),
    },
    Case {
        name: CaseName("group_count_counts_distinct_when_asked"),
        word: Word::Succeeds,
        input: || Scenario::GroupCount,
        // Rows (r1, a), (r1, a), (r1, b), (r2, a) grouped by refinement: three and one rows, two and one distinct names.
        expect: Expect::Value(Cow::Borrowed("r1 3 | r2 1; r1 2 | r2 1")),
    },
    Case {
        name: CaseName("a_meaning_never_reads_past_its_inputs"),
        word: Word::Succeeds,
        input: || Scenario::ReadsOnlyItsInputs,
        // Every lowered kind over `DEFINES`, its inputs holding another relation too: the other relation, the bound tables
        // and the observations are never read; `DEFINES` is.
        expect: Expect::Value(Cow::Borrowed(
            "defines read: true; other reads: 0; bound reads: 0; observation reads: 0",
        )),
    },
    Case {
        name: CaseName("select_keeps_source_row_order_and_duplicate_columns"),
        word: Word::Succeeds,
        input: || Scenario::SelectOrder,
        expect: Expect::Value(Cow::Borrowed("b n0 b | b n2 b")),
    },
    Case {
        name: CaseName("project_keeps_zero_column_and_repeated_rows"),
        word: Word::Succeeds,
        input: || Scenario::ProjectZeroColumns,
        // Three rows of no column, then the names with their repeat.
        expect: Expect::Value(Cow::Borrowed("- | - | -; a | b | a")),
    },
    Case {
        name: CaseName("antijoin_retains_duplicate_when_rhs_absent_and_skips_present_key"),
        word: Word::Succeeds,
        input: || Scenario::AntijoinOfPresentAndAbsentKeys,
        // Names a, b, a against the right's b: both a rows stay, the b row goes.
        expect: Expect::Value(Cow::Borrowed("a | a")),
    },
    Case {
        name: CaseName("empty_right_antijoin_never_reads_invalid_left_key"),
        word: Word::Succeeds,
        input: || Scenario::AntijoinOfNoRightRowsPastItsKey,
        expect: Expect::Value(Cow::Borrowed("a | b | a")),
    },
    Case {
        name: CaseName("nonempty_right_antijoin_checks_first_missing_key"),
        word: Word::Refuses,
        input: || Scenario::AntijoinOfRightRowsPastItsKey,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn {
            index: ColumnIndex(5),
        }),
    },
    Case {
        name: CaseName("joined_rows_preserve_bag_order_duplicate_columns_and_view_mapping"),
        word: Word::Succeeds,
        input: || Scenario::JoinThroughAView,
        // The right read through the view (name, node): its column 0 is the name and 1 the node, repeated in the answer.
        expect: Expect::Value(Cow::Borrowed(
            "n0 n0 b b | n0 n2 b b | n1 n1 a a | n2 n0 b b | n2 n2 b b",
        )),
    },
    Case {
        name: CaseName("empty_matches_do_not_access_invalid_output_columns"),
        word: Word::Succeeds,
        input: || Scenario::JoinOfNoMatchPastItsOutput,
        expect: Expect::Value(Cow::Borrowed("")),
    },
    Case {
        name: CaseName("missing_left_key_is_refused_before_candidate_access"),
        word: Word::Refuses,
        input: || Scenario::JoinPastItsLeftKey,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn {
            index: ColumnIndex(9),
        }),
    },
];

/// The antijoin of node 0's one column of names with the rows of node 1 on their name, the left keyed on `left_key`.
fn antijoin_of_names(left_key: u32, right_name: &str) -> Result<String, Failure> {
    answered(
        &[
            project(relation(), &[3], RepeatedRows::Kept),
            select(relation(), vec![named(right_name)]),
            both(
                Operation::Antijoin {
                    left: input_of(node(0)),
                    right: input_of(node(1)),
                    left_on: vec![ColumnIndex(left_key)],
                    right_on: vec![ColumnIndex(3)],
                },
                node(0),
                node(1),
            ),
        ],
        &[2],
        &[("r1", "a"), ("r1", "b"), ("r1", "a")],
    )
}

/// One cell as the cases show it: a node by its place in file 0, a text by the file's text, a number as itself.
fn cell(value: Cell, texts: &TextResources) -> String {
    if let Some(at) = (0..16).find(|at| value == Cell::Node(node_id(FileId(0), Position(*at)))) {
        return format!("n{at}");
    }
    if let Some(text) = TEXTS.iter().find(|text| texts.symbol(text) == value) {
        return (*text).to_owned();
    }
    match value {
        Cell::Number(Integer(number)) => number.to_string(),
        other => format!("{other:?}"),
    }
}

/// The rows of `table`, a row's cells spaced, rows apart by ` | `; a row of no cell is `-`.
fn shown(table: &Table, texts: &TextResources) -> String {
    let width = table.kinds.len();
    (0..table.count.0)
        .map(|row| {
            if width == 0 {
                return "-".to_owned();
            }
            table
                .cells
                .get(row * width..(row + 1) * width)
                .unwrap_or_default()
                .iter()
                .map(|value| cell(*value, texts))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

/// What `steps` answer at `emits` over `DEFINES` rows of `pairs`, shown node by node.
fn answered(steps: &[Step], emits: &[u32], pairs: &[(&str, &str)]) -> Result<String, Failure> {
    let mut texts = TextResources::new(TEXTS);
    let rows = defines(pairs, &texts);
    let inputs = TableInputs::new(&[(DEFINES, &rows)]);
    let emits: Vec<PlanNodeId> = emits.iter().copied().map(PlanNodeId).collect();
    let tables = emitted(
        steps,
        &emits,
        &inputs,
        &mut texts,
        &mut FileBuffers::default(),
    )?;
    Ok(tables
        .iter()
        .map(|table| shown(table, &texts))
        .collect::<Vec<_>>()
        .join("; "))
}

/// The input of a step that reads `DEFINES`.
fn relation() -> PlanInput {
    PlanInput::Relation(DEFINES)
}

/// The input of a step that reads node `at`.
fn node(at: u32) -> PlanInput {
    PlanInput::Node(PlanNodeId(at))
}

/// A projection of `from`'s `columns`, dropping repeated rows when `distinct` says.
fn project(from: PlanInput, columns: &[u32], distinct: RepeatedRows) -> Step {
    Step {
        operation: Operation::Project {
            from: input_of(from),
            columns: columns.iter().copied().map(ColumnIndex).collect(),
            distinct,
        },
        inputs: vec![from],
    }
}

/// A step reading `left` then `right`.
fn both(operation: Operation, left: PlanInput, right: PlanInput) -> Step {
    Step {
        operation,
        inputs: vec![left, right],
    }
}

/// The join of `left` with `right` on one key each, answering the columns `output` names.
fn join(left: (PlanInput, u32), right: (PlanInput, u32), output: &[(Side, u32)]) -> Step {
    both(
        Operation::Join {
            left: input_of(left.0),
            right: input_of(right.0),
            left_on: vec![ColumnIndex(left.1)],
            right_on: vec![ColumnIndex(right.1)],
            output: output
                .iter()
                .map(|(side, column)| JoinColumn {
                    side: *side,
                    column: ColumnIndex(*column),
                })
                .collect(),
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        },
        left.0,
        right.0,
    )
}

/// The counted grouping of `from` by its refinement, counting the distinct names when `distinct`.
fn group_count(from: PlanInput, distinct: bool) -> Step {
    Step {
        operation: Operation::GroupCount {
            from: input_of(from),
            keys: vec![ColumnIndex(2)],
            distinct: distinct.then_some(ColumnIndex(3)),
        },
        inputs: vec![from],
    }
}

/// The inputs of a run that count what it reads besides the relations it holds.
struct Watched {
    /// The relations held.
    inner: TableInputs,
    /// How many bound tables were read.
    bound: std::cell::Cell<usize>,
    /// How many observations were read.
    observation: std::cell::Cell<usize>,
}

impl PlanInputs for Watched {
    fn facts(&self, relation: Relation) -> FactRows<'_> {
        self.inner.facts(relation)
    }
    fn roots(&self, root: PlanNodeId) -> Candidates<'_> {
        self.inner.roots(root)
    }
    fn bound(&self, parameter: ParameterId) -> &Table {
        self.bound.set(self.bound.get() + 1);
        self.inner.bound(parameter)
    }
    fn observation(&self, input: ObservationBindingId) -> &Table {
        self.observation.set(self.observation.get() + 1);
        self.inner.observation(input)
    }
}

/// Every node of the text program, each run in turn over `DEFINES` beside another relation, its fault kept by the node: what
/// the run read.
fn reads_of_every_kind() -> Result<String, Failure> {
    const OTHER: Relation = Relation::Function;
    let steps = text_program();
    let mut texts = TextResources::new(TEXTS);
    let rows = defines(&[("r1", "a"), ("r2", "b")], &texts);
    let inputs = Watched {
        inner: TableInputs::new(&[(DEFINES, &rows), (OTHER, &rows)]),
        bound: std::cell::Cell::new(0),
        observation: std::cell::Cell::new(0),
    };
    let mut program = LogicalProgram::new();
    for step in &steps {
        let _id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    let nodes: Vec<PlanNodeId> = (0..u32::try_from(steps.len()).unwrap_or(u32::MAX))
        .map(PlanNodeId)
        .collect();
    for (rule, node) in (0..).map(RuleOrdinal).zip(&nodes) {
        program.emit(rule, *node);
    }
    let plan = planned(&program)?;
    let mut buffers = FileBuffers::default();
    for node in nodes {
        // A node's fault is its own answer: what is checked is what the run read to make it.
        let _answer = ran(&plan, node, &mut buffers, &inputs, &mut texts);
    }
    Ok(format!(
        "defines read: {}; other reads: {}; bound reads: {}; observation reads: {}",
        inputs.inner.reads(DEFINES) > 0,
        inputs.inner.reads(OTHER),
        inputs.bound.get(),
        inputs.observation.get()
    ))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let repeated = [("r1", "a"), ("r1", "b"), ("r1", "a")];
    Ok(Cow::Owned(match scenario {
        Scenario::SetOperations => answered(
            &[
                project(relation(), &[3], RepeatedRows::Kept),
                select(relation(), vec![named("b")]),
                project(node(1), &[3], RepeatedRows::Kept),
                both(
                    Operation::Union {
                        left: input_of(node(0)),
                        right: input_of(node(0)),
                    },
                    node(0),
                    node(0),
                ),
                both(
                    Operation::Intersection {
                        left: input_of(node(0)),
                        right: input_of(node(0)),
                    },
                    node(0),
                    node(0),
                ),
                both(
                    Operation::Difference {
                        left: input_of(node(0)),
                        right: input_of(node(2)),
                    },
                    node(0),
                    node(2),
                ),
                project(relation(), &[3], RepeatedRows::Dropped),
                Step {
                    operation: Operation::Extend {
                        from: input_of(node(0)),
                        values: vec![Operand::Literal(QueryLiteral::Number(Integer(1)))],
                    },
                    inputs: vec![node(0)],
                },
            ],
            &[0, 3, 4, 5, 6, 7],
            &repeated,
        )?,
        Scenario::AntijoinWithNoRightRows => answered(
            &[
                project(relation(), &[3], RepeatedRows::Kept),
                select(relation(), vec![named("zzz")]),
                both(
                    Operation::Antijoin {
                        left: input_of(node(0)),
                        right: input_of(node(1)),
                        left_on: vec![ColumnIndex(0)],
                        right_on: vec![ColumnIndex(3)],
                    },
                    node(0),
                    node(1),
                ),
            ],
            &[2],
            &repeated,
        )?,
        Scenario::JoinOrder => answered(
            &[join(
                (relation(), 3),
                (relation(), 3),
                &[(Side::Left, 0), (Side::Right, 0)],
            )],
            &[0],
            &[("r1", "b"), ("r1", "a"), ("r1", "b")],
        )?,
        Scenario::FactsAndTable => {
            let pairs = [("r1", "b"), ("r1", "a"), ("r2", "b")];
            // Node 0 holds every row of `DEFINES` as a table; each pair of steps reads the relation, then node 0.
            let every = project(relation(), &[0, 1, 2, 3, 4, 5, 6], RepeatedRows::Kept);
            let over = |from: PlanInput| {
                [
                    Step {
                        operation: Operation::Select {
                            from: input_of(from),
                            comparisons: vec![named("a")],
                            kept: vec![ColumnIndex(0), ColumnIndex(3)],
                        },
                        inputs: vec![from],
                    },
                    project(from, &[0, 3], RepeatedRows::Kept),
                    group_count(from, false),
                    join((from, 3), (from, 3), &[(Side::Left, 0), (Side::Right, 0)]),
                ]
            };
            let mut steps = vec![every];
            steps.extend(over(relation()));
            steps.extend(over(node(0)));
            let facts = answered(&steps, &[1, 2, 3, 4], &pairs)?;
            let table = answered(&steps, &[5, 6, 7, 8], &pairs)?;
            if facts == table {
                facts
            } else {
                format!("facts {facts} != table {table}")
            }
        }
        Scenario::GroupCount => answered(
            &[
                group_count(relation(), false),
                group_count(relation(), true),
            ],
            &[0, 1],
            &[("r1", "a"), ("r1", "a"), ("r1", "b"), ("r2", "a")],
        )?,
        Scenario::ReadsOnlyItsInputs => reads_of_every_kind()?,
        Scenario::SelectOrder => answered(
            &[Step {
                operation: Operation::Select {
                    from: input_of(relation()),
                    comparisons: vec![named("b")],
                    kept: vec![ColumnIndex(3), ColumnIndex(0), ColumnIndex(3)],
                },
                inputs: vec![relation()],
            }],
            &[0],
            &[("r1", "b"), ("r1", "a"), ("r1", "b")],
        )?,
        Scenario::AntijoinOfPresentAndAbsentKeys => antijoin_of_names(0, "b")?,
        Scenario::AntijoinOfNoRightRowsPastItsKey => antijoin_of_names(5, "zzz")?,
        Scenario::AntijoinOfRightRowsPastItsKey => antijoin_of_names(5, "b")?,
        Scenario::JoinThroughAView => {
            let mut through = join(
                (relation(), 3),
                (relation(), 0),
                &[
                    (Side::Left, 0),
                    (Side::Right, 1),
                    (Side::Right, 0),
                    (Side::Left, 3),
                ],
            );
            if let Operation::Join { view, .. } = &mut through.operation {
                *view = vec![ColumnIndex(3), ColumnIndex(0)];
            }
            answered(&[through], &[0], &[("r1", "b"), ("r1", "a"), ("r1", "b")])?
        }
        Scenario::JoinOfNoMatchPastItsOutput => answered(
            &[
                select(relation(), vec![named("zzz")]),
                join(
                    (relation(), 3),
                    (node(0), 3),
                    &[(Side::Left, 0), (Side::Right, 99)],
                ),
            ],
            &[1],
            &repeated,
        )?,
        Scenario::JoinPastItsLeftKey => answered(
            &[join((relation(), 9), (relation(), 3), &[(Side::Left, 0)])],
            &[0],
            &repeated,
        )?,
        Scenario::ProjectZeroColumns => answered(
            &[
                project(relation(), &[], RepeatedRows::Kept),
                project(relation(), &[3], RepeatedRows::Kept),
            ],
            &[0, 1],
            &repeated,
        )?,
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
