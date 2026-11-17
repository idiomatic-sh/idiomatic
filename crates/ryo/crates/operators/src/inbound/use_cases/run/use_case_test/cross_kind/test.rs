//! What every kind keeps across kinds, through `plan` and `run` only: which kinds answer a row once and which keep its
//! repeats, the order a select, a project and a join answer in, an antijoin over no right rows, a counted grouping's
//! distinct count, the same rows from a relation's columns and from an earlier node's table, and that a program reads only
//! the inputs its steps name. The antijoin's and the join's own cases are `relational`'s.
use super::super::super::fixtures::cases::{DEFINES, defines, input_of, named, select};
use super::super::super::fixtures::relational::{
    TEXTS, answered, both, group_count, join, node, project, relation,
};
use super::super::super::fixtures::{TableInputs, TextResources, planned, ran};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::fixtures::text::text_program;
use crate::domain::physical_plan::types::bound_observation_place::value::BoundObservationPlace;
use crate::domain::physical_plan::types::candidates::value::Candidates;
use crate::domain::physical_plan::types::fact_rows::value::FactRows;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::frontier_place::value::FrontierPlace;
use crate::domain::physical_plan::types::held_rows::value::HeldRows;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, PlanInput, PlanInputList, PlanInputs, PlanNodeId,
};
use ryo_relations::domain::relations::types::{Integer, Relation};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, HostedNumber, Operand, QueryLiteral, RepeatedRows, Side, StepScope,
};
use ryo_values::domain::plans::types::Operation;
use std::borrow::Cow;

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
        // Every lowered kind over `DEFINES`, its inputs holding another relation too: the other relation, the frontiers, the bound
        // observations and the host's tables are never read; `DEFINES` is.
        expect: Expect::Value(Cow::Borrowed(
            "defines read: true; other reads: 0; frontier reads: 0; observation reads: 0; hosted reads: 0",
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
];

/// The inputs of a run that count what it reads besides the relations it holds.
struct Watched {
    /// The relations held.
    inner: TableInputs,
    /// How many frontiers, bound observations and host tables were read, in that order.
    held: [std::cell::Cell<usize>; 3],
}

impl Watched {
    /// Counts one read of the held input at `at`.
    fn read(&self, at: usize) {
        if let Some(count) = self.held.get(at) {
            count.set(count.get() + 1);
        }
    }
}

impl PlanInputs for Watched {
    fn facts(&self, relation: Relation) -> FactRows<'_> {
        self.inner.facts(relation)
    }
    fn roots(&self, root: PlanNodeId) -> Candidates<'_> {
        self.inner.roots(root)
    }
    fn frontier(&self, place: FrontierPlace) -> HeldRows<'_> {
        self.read(0);
        self.inner.frontier(place)
    }
    fn observation(&self, place: BoundObservationPlace) -> HeldRows<'_> {
        self.read(1);
        self.inner.observation(place)
    }
    fn hosted(&self, relation: HostedNumber) -> HeldRows<'_> {
        self.read(2);
        self.inner.hosted(relation)
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
        held: Default::default(),
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
        "defines read: {}; other reads: {}; frontier reads: {}; observation reads: {}; hosted reads: {}",
        inputs.inner.reads(DEFINES) > 0,
        inputs.inner.reads(OTHER),
        inputs.held[0].get(),
        inputs.held[1].get(),
        inputs.held[2].get()
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
