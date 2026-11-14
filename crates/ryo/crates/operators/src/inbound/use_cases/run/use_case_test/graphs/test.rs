//! The graph kinds through `plan` and `run` only: what a closure, a closure fused into its join and a group digest answer,
//! and what a seeded closure walks, against the same rows run another way or recorded from main's row meaning.
use super::super::super::fixtures::{TableInputs, TextResources, emitted};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{FileId, Integer, Position, Relation, Value as Cell};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as StepInput, JoinColumn, ReachLookup, Side,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// The relation the cases read edges from: parent, child, ordinal, field, file.
const EDGES: Relation = Relation::SemanticChild;

#[derive(Clone, Copy)]
enum Scenario {
    WholeClosure,
    SeededAsWhole,
    SeededAsWholeWithResidualKey,
    SeededWalksOnlyBound,
    BoundWalkedOnce,
    GroupDigest,
}
#[derive(Debug, PartialEq)]
enum Observation {
    /// The rows answered, each cell a node's position or a number.
    Rows(std::borrow::Cow<'static, [i64]>),
    /// Whether the two runs a case compares answered the same.
    Same(bool),
    /// How many starts a seeded closure walked, and how many entities they reached.
    Walked(usize, usize),
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_closure_reaches_each_entity_once_ascending"),
        word: Word::Succeeds,
        input: || Scenario::WholeClosure,
        // Sources 1, 2, 3 and 5 ascending, each reaching 1 to 4 ascending once, over a cycle and a repeated edge.
        expect: Expect::Value(Observation::Rows(std::borrow::Cow::Borrowed(&[
            1, 1, 1, 2, 1, 3, 1, 4, 2, 1, 2, 2, 2, 3, 2, 4, 3, 1, 3, 2, 3, 3, 3, 4, 5, 1, 5, 2, 5, 3, 5, 4,
        ]))),
    },
    Case {
        name: CaseName("a_seeded_closure_answers_what_the_join_over_the_whole_closure_answers"),
        word: Word::Succeeds,
        input: || Scenario::SeededAsWhole,
        expect: Expect::Value(Observation::Same(true)),
    },
    Case {
        name: CaseName("a_seeded_closure_answers_what_the_join_over_the_whole_closure_answers_with_a_residual_key"),
        word: Word::Succeeds,
        input: || Scenario::SeededAsWholeWithResidualKey,
        expect: Expect::Value(Observation::Same(true)),
    },
    Case {
        name: CaseName("a_seeded_closure_never_walks_an_unbound_source"),
        word: Word::Succeeds,
        input: || Scenario::SeededWalksOnlyBound,
        expect: Expect::Value(Observation::Walked(2, 20)),
    },
    Case {
        name: CaseName("a_bound_source_is_walked_once"),
        word: Word::Succeeds,
        input: || Scenario::BoundWalkedOnce,
        expect: Expect::Value(Observation::Walked(1, 10)),
    },
    Case {
        name: CaseName("a_group_digest_is_the_row_meaning_s"),
        word: Word::Succeeds,
        input: || Scenario::GroupDigest,
        // Per group in first-row order: its key, then the digest and count main's reduction answers for the same rows
        // (`reduction` service_test `a_group_digest_s_row_meaning_is_first_row_distinct_count_and_digest_per_group`).
        expect: Expect::Value(Observation::Rows(std::borrow::Cow::Borrowed(&[
            1,
            -5_686_229_789_459_316_394,
            2,
            2,
            -5_928_100_084_692_852_672,
            1,
            3,
            7_069_949_220_488_350_037,
            1,
        ]))),
    },
];

/// The node at position `at` of file 0.
fn node(at: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(at)))
}

/// The edge rows `parent -> child`, each with its ordinal, the field `f` and file 0.
fn edges(pairs: &[(u32, u32, i64)], texts: &TextResources) -> Table {
    let mut cells = Vec::with_capacity(pairs.len() * kinds(EDGES).len());
    for (parent, child, ordinal) in pairs {
        cells.extend([
            node(*parent),
            node(*child),
            Cell::Number(Integer(*ordinal)),
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

/// A cell as the case states it: a node by its position, a number as itself.
fn stated(cell: Cell) -> i64 {
    match cell {
        Cell::Node(id) => i64::from(ryo_relations::domain::relations::service::node_position(id).0),
        Cell::Number(Integer(number)) => number,
        other => panic!("a case states nodes and numbers only, not {other:?}"),
    }
}

/// The closure of the edges from every source.
fn closure(edges: StepInput, input: PlanInput) -> Step {
    Step {
        operation: Operation::Closure {
            edges,
            keys: Vec::new(),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        inputs: vec![input],
    }
}

/// A join of the edges, as left rows, with `right` on the left's parent and, with `residual`, its child, answering the
/// left's parent and the right's destination; `reach` when the closure was fused into it.
fn join(right: (StepInput, PlanInput), residual: bool, reach: Option<ReachLookup>) -> Step {
    let (left_on, right_on) = if residual {
        (vec![ColumnIndex(0), ColumnIndex(1)], vec![ColumnIndex(0), ColumnIndex(1)])
    } else {
        (vec![ColumnIndex(0)], vec![ColumnIndex(0)])
    };
    Step {
        operation: Operation::Join {
            left: StepInput::Relation(EDGES),
            right: right.0,
            left_on,
            right_on,
            output: vec![
                JoinColumn { side: Side::Left, column: ColumnIndex(0) },
                JoinColumn { side: Side::Right, column: ColumnIndex(1) },
            ],
            filter: Vec::new(),
            view: Vec::new(),
            reach,
        },
        inputs: vec![PlanInput::Relation(EDGES), right.1],
    }
}

/// The closure's demand as the query planner fuses it: no partition, its source then its destination.
fn reach() -> ReachLookup {
    ReachLookup {
        source: ColumnIndex(0),
        destination: ColumnIndex(1),
        keys: Vec::new(),
        columns: vec![ColumnIndex(0), ColumnIndex(1)],
    }
}

/// What the join over the whole closure and the seeded closure answer over the same edges.
fn seeded_as_whole(residual: bool) -> Result<Value, Failure> {
    let mut resources = TextResources::new(&["f"]);
    let table = edges(&[(1, 2, 0), (2, 3, 0), (3, 1, 0), (1, 4, 1), (5, 1, 0), (1, 2, 2), (6, 7, 0)], &resources);
    let inputs = TableInputs::new(&[(EDGES, &table)]);
    let whole = [
        closure(StepInput::Relation(EDGES), PlanInput::Relation(EDGES)),
        join((StepInput::Step(ryo_values::domain::plan_parts::types::StepId(0)), PlanInput::Node(PlanNodeId(0))), residual, None),
    ];
    let seeded = [join((StepInput::Relation(EDGES), PlanInput::Relation(EDGES)), residual, Some(reach()))];
    let over_whole = emitted(&whole, &[PlanNodeId(1)], &inputs, &mut resources, &mut FileBuffers::default())?;
    let over_seeds = emitted(&seeded, &[PlanNodeId(0)], &inputs, &mut resources, &mut FileBuffers::default())?;
    Ok(Observation::Same(over_whole == over_seeds && over_whole.first().is_some_and(|rows| rows.count.0 > 0)))
}

/// How many starts and entities a seeded closure over `chains` chains of `length` edges walked, its left rows binding the
/// first node of each chain in `bound`, `repeats` times each.
fn walked(chains: u32, length: u32, bound: &[u32], repeats: usize) -> Result<Value, Failure> {
    let mut resources = TextResources::new(&["f"]);
    let mut pairs = Vec::new();
    for chain in 0..chains {
        let first = chain * (length + 1);
        pairs.extend((first..first + length).map(|at| (at, at + 1, 0)));
    }
    let table = edges(&pairs, &resources);
    let starts: Vec<(u32, u32, i64)> = bound
        .iter()
        .flat_map(|chain| std::iter::repeat_n((chain * (length + 1), 0, 0), repeats))
        .collect();
    let left = edges(&starts, &resources);
    let inputs = TableInputs::new(&[(EDGES, &table), (Relation::Defines, &left)]);
    let mut step = join((StepInput::Relation(EDGES), PlanInput::Relation(EDGES)), false, Some(reach()));
    if let Operation::Join { left, .. } = &mut step.operation {
        *left = StepInput::Relation(Relation::Defines);
    }
    step.inputs[0] = PlanInput::Relation(Relation::Defines);
    let mut buffers = FileBuffers::default();
    let _rows = emitted(&[step], &[PlanNodeId(0)], &inputs, &mut resources, &mut buffers)?;
    Ok(Observation::Walked(buffers.walk.runs.len(), buffers.walk.reached.len()))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::WholeClosure => {
            let mut resources = TextResources::new(&["f"]);
            let table = edges(&[(3, 1, 0), (1, 2, 0), (2, 3, 0), (1, 4, 1), (5, 1, 0), (1, 2, 2)], &resources);
            let inputs = TableInputs::new(&[(EDGES, &table)]);
            let steps = [closure(StepInput::Relation(EDGES), PlanInput::Relation(EDGES))];
            let rows = emitted(&steps, &[PlanNodeId(0)], &inputs, &mut resources, &mut FileBuffers::default())?;
            Ok(Observation::Rows(rows.iter().flat_map(|table| table.cells.iter().copied().map(stated)).collect()))
        }
        Scenario::SeededAsWhole => seeded_as_whole(false),
        Scenario::SeededAsWholeWithResidualKey => seeded_as_whole(true),
        Scenario::SeededWalksOnlyBound => walked(1_000, 10, &[3, 700], 1),
        Scenario::BoundWalkedOnce => walked(10, 10, &[4], 100),
        Scenario::GroupDigest => {
            let mut resources = TextResources::new(&["f"]);
            let table = edges(&[(1, 0, 7), (2, 0, 9), (1, 0, 8), (1, 0, 7), (2, 0, 9), (3, 0, 4)], &resources);
            let inputs = TableInputs::new(&[(EDGES, &table)]);
            let steps = [Step {
                operation: Operation::GroupDigest {
                    from: StepInput::Relation(EDGES),
                    keys: vec![ColumnIndex(0)],
                    over: ColumnIndex(2),
                },
                inputs: vec![PlanInput::Relation(EDGES)],
            }];
            let rows = emitted(&steps, &[PlanNodeId(0)], &inputs, &mut resources, &mut FileBuffers::default())?;
            Ok(Observation::Rows(rows.iter().flat_map(|table| table.cells.iter().copied().map(stated)).collect()))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
