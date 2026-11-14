//! The operators crate's kernels workload.
//!
//! One file's rows of `defines`, planned and run through seam C at a few row counts: selects keyed on one name column (one
//! shared index and its lookups), a projection, a union, an antijoin, an extension, a join and a counted grouping over them;
//! three probes of one right side and key, with their index shared and each with its own; the nodes that drop repeated rows;
//! and two text kinds over every row's span. Recorded, not gated.

#![expect(
    clippy::expect_used,
    reason = "a bench that cannot run its own program has nothing to measure: the panic is the failure"
)]

use ryo_operators::domain::physical_plan::fixtures::Step;
use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_operators::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::physical_plan::types::{PlanInputs, Resources};
use ryo_operators::inbound::use_cases::run::fixtures::unshared::emitted_unshared;
use ryo_operators::inbound::use_cases::run::fixtures::{TableInputs, TextResources, emitted};
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{
    FileId, Integer, Offset, Position, Relation, Span, Value,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input, JoinColumn, LiteralText, Operand, PlanComparison,
    QueryLiteral, RepeatedRows, Side, StepId, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// The relation the workload reads: refinement (column 2) and name (column 3) are texts.
const DEFINES: Relation = Relation::Defines;

/// How many distinct names the rows hold, and how many selects key on one of them.
const NAMES: usize = 43;

/// The rows of `DEFINES`: row `at` names `names[at % NAMES]`, every third one refined as `other`.
fn defines(rows: usize, names: &[String], texts: &TextResources) -> Table {
    let mut cells = Vec::with_capacity(rows * kinds(DEFINES).len());
    for at in 0..rows {
        let tree = Value::Node(node_id(
            FileId(0),
            Position(u32::try_from(at).unwrap_or(u32::MAX)),
        ));
        let name = names.get(at % NAMES).map_or("kind", String::as_str);
        cells.extend([
            tree,
            texts.symbol("kind"),
            texts.symbol(if at % 3 == 0 { "other" } else { "kind" }),
            texts.symbol(name),
            tree,
            Value::File(FileId(0)),
            Value::Span(Span {
                start: Offset(0),
                end: Offset(0),
            }),
        ]);
    }
    Table {
        kinds: kinds(DEFINES).to_vec(),
        cells,
        count: RowCount(rows),
    }
}

/// A comparison of column `column` with the written text `text`.
fn equal(column: u32, text: &str) -> PlanComparison {
    PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// The step of `operation` over `inputs`.
fn step(operation: Operation, inputs: Vec<PlanInput>) -> Step {
    Step { operation, inputs }
}

/// The input naming node `at`, as the operation and the program name it.
fn node(at: usize) -> (Input, PlanInput) {
    let at = u32::try_from(at).unwrap_or(u32::MAX);
    (Input::Step(StepId(at)), PlanInput::Node(PlanNodeId(at)))
}

/// The workload's program: a select keyed on each name, then the other kinds over the first two of them.
fn program(names: &[String]) -> Vec<Step> {
    let relation = (Input::Relation(DEFINES), PlanInput::Relation(DEFINES));
    let mut steps: Vec<Step> = names
        .iter()
        .map(|name| {
            step(
                Operation::Select {
                    from: relation.0,
                    comparisons: vec![equal(3, name)],
                    kept: Vec::new(),
                },
                vec![relation.1],
            )
        })
        .collect();
    let first = steps.len();
    let projected = |at: usize| {
        step(
            Operation::Project {
                from: node(at).0,
                columns: vec![ColumnIndex(3)],
                distinct: RepeatedRows::Dropped,
            },
            vec![node(at).1],
        )
    };
    steps.extend([
        projected(0),
        projected(1),
        step(
            Operation::Union {
                left: node(first).0,
                right: node(first + 1).0,
            },
            vec![node(first).1, node(first + 1).1],
        ),
        step(
            Operation::Antijoin {
                left: relation.0,
                right: node(first + 2).0,
                left_on: vec![ColumnIndex(3)],
                right_on: vec![ColumnIndex(0)],
            },
            vec![relation.1, node(first + 2).1],
        ),
        step(
            Operation::Extend {
                from: node(0).0,
                values: vec![Operand::Literal(QueryLiteral::Number(Integer(1)))],
            },
            vec![node(0).1],
        ),
        step(
            Operation::Join {
                left: node(first + 2).0,
                right: relation.0,
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(3)],
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(0),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(0),
                    },
                ],
                filter: vec![equal(2, "other")],
                view: Vec::new(),
                reach: None,
            },
            vec![node(first + 2).1, relation.1],
        ),
        step(
            Operation::GroupCount {
                from: relation.0,
                keys: vec![ColumnIndex(3)],
                distinct: None,
            },
            vec![relation.1],
        ),
    ]);
    steps
}

/// How a bench row plans and runs its program: as the planner plans it, or with every probe on an index of its own.
type Runner = fn(
    &[Step],
    &[PlanNodeId],
    &dyn PlanInputs,
    &mut dyn Resources,
    &mut FileBuffers,
) -> Result<Vec<Table>, PortableOperatorFault>;

/// `steps` over `rows` rows of `DEFINES`, run by `runner`: every node emitted, its buffers released after.
fn measured(bencher: divan::Bencher<'_, '_>, rows: usize, steps: &dyn Fn(&[String]) -> Vec<Step>, runner: Runner) {
    let names: Vec<String> = (0..NAMES).map(|at| format!("name{at}")).collect();
    let texts: Vec<&str> = ["kind", "other"]
        .into_iter()
        .chain(names.iter().map(String::as_str))
        .collect();
    let mut resources = TextResources::new(&texts);
    let table = defines(rows, &names, &resources);
    let inputs = TableInputs::new(&[(DEFINES, &table)]);
    let steps = steps(&names);
    let emits: Vec<PlanNodeId> = (0..steps.len())
        .map(|at| PlanNodeId(u32::try_from(at).unwrap_or(u32::MAX)))
        .collect();
    let mut buffers = FileBuffers::default();
    bencher.bench_local(|| {
        let answered = runner(&steps, &emits, &inputs, &mut resources, &mut buffers)
            .expect("the workload's program runs");
        for emit in &emits {
            buffers.release(*emit);
        }
        answered
    });
}

/// The workload over `rows` rows: the program planned and run, every node emitted, its buffers released after.
#[divan::bench(args = [1_000, 100_000])]
fn planned_and_run(bencher: divan::Bencher<'_, '_>, rows: usize) {
    measured(bencher, rows, &program, emitted);
}

/// A select of the first name and a projection of name and node, then three joins with `DEFINES` on its node: of `DEFINES`,
/// of the select and of the projection, each answering other columns. One right side and key: the planner shares its index.
fn probes(names: &[String]) -> Vec<Step> {
    let relation = (Input::Relation(DEFINES), PlanInput::Relation(DEFINES));
    let join = |left: (Input, PlanInput), left_on: u32, output: &[(Side, u32)]| {
        step(
            Operation::Join {
                left: left.0,
                right: relation.0,
                left_on: vec![ColumnIndex(left_on)],
                right_on: vec![ColumnIndex(0)],
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
            vec![left.1, relation.1],
        )
    };
    let first = names.first().map_or("kind", String::as_str);
    vec![
        step(
            Operation::Select {
                from: relation.0,
                comparisons: vec![equal(3, first)],
                kept: Vec::new(),
            },
            vec![relation.1],
        ),
        step(
            Operation::Project {
                from: relation.0,
                columns: vec![ColumnIndex(3), ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
            vec![relation.1],
        ),
        join(relation, 0, &[(Side::Left, 3), (Side::Right, 2)]),
        join(node(0), 0, &[(Side::Right, 3)]),
        join(node(1), 1, &[(Side::Left, 0), (Side::Right, 6)]),
    ]
}

/// The three probes with one shared index, built once per file.
#[divan::bench(args = [10_000, 100_000])]
fn shared_probes(bencher: divan::Bencher<'_, '_>, rows: usize) {
    measured(bencher, rows, &probes, emitted);
}

/// The three probes, each building its own index.
#[divan::bench(args = [10_000, 100_000])]
fn own_probes(bencher: divan::Bencher<'_, '_>, rows: usize) {
    measured(bencher, rows, &probes, emitted_unshared);
}

/// A union of `DEFINES` with itself, an intersection of it with itself, and its names with repeats dropped.
fn repeats(_names: &[String]) -> Vec<Step> {
    let relation = (Input::Relation(DEFINES), PlanInput::Relation(DEFINES));
    vec![
        step(
            Operation::Union {
                left: relation.0,
                right: relation.0,
            },
            vec![relation.1, relation.1],
        ),
        step(
            Operation::Intersection {
                left: relation.0,
                right: relation.0,
            },
            vec![relation.1, relation.1],
        ),
        step(
            Operation::Project {
                from: relation.0,
                columns: vec![ColumnIndex(3)],
                distinct: RepeatedRows::Dropped,
            },
            vec![relation.1],
        ),
    ]
}

/// The nodes that drop repeated rows, through `hash_index`'s keyed rows.
#[divan::bench(args = [1_000, 100_000])]
fn repeats_dropped(bencher: divan::Bencher<'_, '_>, rows: usize) {
    measured(bencher, rows, &repeats, emitted);
}

/// Source-matches of `\A` and span-bounds over every row's span.
fn spans(_names: &[String]) -> Vec<Step> {
    let relation = (Input::Relation(DEFINES), PlanInput::Relation(DEFINES));
    vec![
        step(
            Operation::SourceMatches {
                from: relation.0,
                span: ColumnIndex(6),
                base_line: None,
                pattern: LiteralText("\\A".into()),
                capture: None,
                capture_text: TextOutput::Omitted,
                capture_line: LineOutput::Omitted,
            },
            vec![relation.1],
        ),
        step(
            Operation::SpanBounds {
                from: relation.0,
                span: ColumnIndex(6),
            },
            vec![relation.1],
        ),
    ]
}

/// Two `text_map` stages over every row's span.
#[divan::bench(args = [1_000, 100_000])]
fn text_kinds(bencher: divan::Bencher<'_, '_>, rows: usize) {
    measured(bencher, rows, &spans, emitted);
}
