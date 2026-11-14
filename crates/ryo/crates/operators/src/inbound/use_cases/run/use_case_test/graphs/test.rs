//! The graph kinds through `plan` and `run` only: what a closure, a closure fused into its join and a group digest answer,
//! and what a seeded closure walks, against the same rows run another way or recorded from main's row meaning.
use super::super::super::fixtures::{TableInputs, TextResources, emitted};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{Step, ordered_existence_pair, ordered_rows};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{FileId, Integer, Position, Relation, Value as Cell};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, JoinColumn, Operand, PlanComparison, ReachLookup, Side, Test,
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
    Cycles,
    OrderedWalk,
    OrderedWalkOverATextOrder,
    OrderedExistence,
    EdgesFaultFirst,
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
    Case {
        name: CaseName("cycles_are_marked_by_tarjan"),
        word: Word::Succeeds,
        input: || Scenario::Cycles,
        // The loop's 1 and 2, the self edge's 3 and the ring's 7, 8 and 9, ascending, each as source and destination; not the
        // chain's 4, 5 and 6 nor the tail's 10.
        expect: Expect::Value(Observation::Rows(std::borrow::Cow::Borrowed(&[1, 1, 2, 2, 3, 3, 7, 7, 8, 8, 9, 9]))),
    },
    Case {
        name: CaseName("an_ordered_walk_follows_the_order_column"),
        word: Word::Succeeds,
        input: || Scenario::OrderedWalk,
        // From roots 1, 2 and 3 (1 twice among the rows, visited once): root, source, destination, edge, ordinal. A source's
        // edges in ascending order, an edge to a met entity yielded but not expanded, ordinals from 0 per root.
        expect: Expect::Value(Observation::Rows(std::borrow::Cow::Borrowed(&[
            1, 1, 2, 1, 0, 1, 2, 3, 0, 1, 1, 3, 1, 0, 2, 1, 1, 3, 2, 3, //
            2, 2, 3, 0, 0, 2, 3, 1, 0, 1, 2, 1, 2, 1, 2, 2, 1, 3, 2, 3, //
            3, 3, 1, 0, 0, 3, 1, 2, 1, 1, 3, 2, 3, 0, 2, 3, 1, 3, 2, 3,
        ]))),
    },
    Case {
        name: CaseName("an_antijoin_of_a_later_row_keeps_each_partition_s_extremum"),
        word: Word::Succeeds,
        input: || Scenario::OrderedExistence,
        // Parent, child and ordinal of each row no row of its parent orders past: the greatest ordinal per parent (both
        // children tied at 2 under 1), then the least (both tied at 3 under 8). The planner rewrote both antijoins to
        // ordered probes of the edges.
        expect: Expect::Value(Observation::Rows(std::borrow::Cow::Borrowed(&[
            1, 3, 2, 1, 4, 2, 6, 7, 5, 8, 11, 4, //
            1, 2, 0, 6, 7, 5, 8, 9, 3, 8, 10, 3,
        ]))),
    },
    Case {
        name: CaseName("a_graph_kernel_raises_its_edges_fault_first"),
        word: Word::Succeeds,
        input: || Scenario::EdgesFaultFirst,
        expect: Expect::Value(Observation::Same(true)),
    },
    Case {
        name: CaseName("an_ordered_walk_over_an_order_that_is_not_a_number_is_refused"),
        word: Word::Refuses,
        input: || Scenario::OrderedWalkOverATextOrder,
        expect: Expect::Error(PortableOperatorFault::ShapeMismatch),
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
        cells.extend([node(*parent), node(*child), Cell::Number(Integer(*ordinal)), texts.symbol("f"), Cell::File(FileId(0))]);
    }
    Table { kinds: kinds(EDGES).to_vec(), cells, count: RowCount(pairs.len()) }
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
        operation: Operation::Closure { edges, keys: Vec::new(), source: ColumnIndex(0), destination: ColumnIndex(1) },
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

/// The join `step` with its left rows read from `left` instead.
fn left_of(mut step: Step, left: (StepInput, PlanInput)) -> Step {
    if let Operation::Join { left: read, .. } = &mut step.operation {
        *read = left.0;
    }
    if let Some(input) = step.inputs.first_mut() {
        *input = left.1;
    }
    step
}

/// The closure's demand as the query planner fuses it: no partition, its source then its destination.
fn reach() -> ReachLookup {
    ReachLookup { source: ColumnIndex(0), destination: ColumnIndex(1), keys: Vec::new(), columns: vec![ColumnIndex(0), ColumnIndex(1)] }
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
    let step = left_of(
        join((StepInput::Relation(EDGES), PlanInput::Relation(EDGES)), false, Some(reach())),
        (StepInput::Relation(Relation::Defines), PlanInput::Relation(Relation::Defines)),
    );
    let mut buffers = FileBuffers::default();
    let _rows = emitted(&[step], &[PlanNodeId(0)], &inputs, &mut resources, &mut buffers)?;
    Ok(Observation::Walked(buffers.walk.runs.len(), buffers.walk.reached.len()))
}

/// The rows `step` answers over `pairs` as the edges, each cell as the case states it.
fn stated_rows(step: Step, pairs: &[(u32, u32, i64)]) -> Result<Value, Failure> {
    let mut resources = TextResources::new(&["f"]);
    let table = edges(pairs, &resources);
    let inputs = TableInputs::new(&[(EDGES, &table)]);
    let rows = emitted(&[step], &[PlanNodeId(0)], &inputs, &mut resources, &mut FileBuffers::default())?;
    Ok(Observation::Rows(rows.iter().flat_map(|table| table.cells.iter().copied().map(stated)).collect()))
}

/// An ordered depth-first visit from every parent of the edges, the edges ordered by `order`, each visit naming its edge by
/// its ordinal column.
fn ordered_walk(order: ColumnIndex) -> Step {
    Step {
        operation: Operation::OrderedDfs {
            from: StepInput::Relation(EDGES),
            edges: StepInput::Relation(EDGES),
            root: ColumnIndex(0),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
            order,
            edge: ColumnIndex(2),
            root_keys: Vec::new(),
            edge_keys: Vec::new(),
        },
        inputs: vec![PlanInput::Relation(EDGES), PlanInput::Relation(EDGES)],
    }
}

/// The rows of `relation` whose field (column 3) self-matches as a regex: a field that is no pattern faults on its row.
fn self_matching(relation: Relation) -> Step {
    Step {
        operation: Operation::Select {
            from: StepInput::Relation(relation),
            comparisons: vec![PlanComparison {
                test: Test::Matches,
                left: Operand::Column(ColumnIndex(3)),
                right: Operand::Column(ColumnIndex(3)),
                origin: ComparisonOrigin::Unrecorded,
            }],
            kept: Vec::new(),
        },
        inputs: vec![PlanInput::Relation(relation)],
    }
}

/// A closure fused into its join reads its left rows (a select failing on row 1) as they stream and its edges (a select
/// failing on row 4) whole before it walks: it reports the edges' fault, the one their select alone reports, and not the
/// left rows' fault of an earlier row.
fn edges_fault_first() -> Result<Value, Failure> {
    let mut resources = TextResources::new(&["f", "(", "["]);
    let fields = |bad: usize, text: &str| {
        let mut table = edges(&[(1, 2, 0), (2, 3, 0), (3, 4, 0), (4, 5, 0), (5, 6, 0), (6, 7, 0)], &resources);
        if let Some(field) = table.cells.get_mut(bad * 5 + 3) {
            *field = resources.symbol(text);
        }
        table
    };
    let (left_rows, edge_rows) = (fields(1, "("), fields(4, "["));
    let inputs = TableInputs::new(&[(EDGES, &edge_rows), (Relation::Defines, &left_rows)]);
    let node = |at| (StepInput::Step(ryo_values::domain::plan_parts::types::StepId(at)), PlanInput::Node(PlanNodeId(at)));
    let seeded = left_of(join(node(1), false, Some(reach())), node(0));
    let (left, edges) = (self_matching(Relation::Defines), self_matching(EDGES));
    let mut run = |steps: &[Step]| {
        let last = PlanNodeId(u32::try_from(steps.len() - 1).unwrap_or(u32::MAX));
        emitted(steps, &[last], &inputs, &mut resources, &mut FileBuffers::default())
    };
    let both = run(&[left.clone(), edges.clone(), seeded]);
    let Err(first) = run(&[edges]) else {
        return Ok(Observation::Same(false));
    };
    let Err(streamed) = run(&[left]) else {
        return Ok(Observation::Same(false));
    };
    Ok(Observation::Same(both == Err(first.clone()) && streamed != first))
}

/// The edges of the ordered walks: 1 leads to 3 (order 2) and to 2 (order 1), 2 to 3, 3 back to 1.
const WALKED: &[(u32, u32, i64)] = &[(1, 3, 2), (1, 2, 1), (2, 3, 0), (3, 1, 0)];

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Cycles => stated_rows(
            Step {
                operation: Operation::Cycles { edges: StepInput::Relation(EDGES), source: ColumnIndex(0), destination: ColumnIndex(1) },
                inputs: vec![PlanInput::Relation(EDGES)],
            },
            // A loop of two, a self edge, a chain, and a ring of three with a tail.
            &[(1, 2, 0), (2, 1, 0), (3, 3, 0), (4, 5, 0), (5, 6, 0), (9, 7, 0), (7, 8, 0), (8, 9, 0), (9, 10, 0)],
        ),
        Scenario::OrderedWalk => stated_rows(ordered_walk(ColumnIndex(2)), WALKED),
        Scenario::EdgesFaultFirst => edges_fault_first(),
        Scenario::OrderedExistence => {
            let mut resources = TextResources::new(&["f"]);
            let table = ordered_rows(resources.symbol("f"));
            let inputs = TableInputs::new(&[(EDGES, &table)]);
            let rows = emitted(&ordered_existence_pair(), &[PlanNodeId(3), PlanNodeId(7)], &inputs, &mut resources, &mut FileBuffers::default())?;
            Ok(Observation::Rows(
                rows.iter().flat_map(|table| table.cells.chunks(5).flat_map(|row| row.iter().take(3).copied().map(stated))).collect(),
            ))
        }
        Scenario::OrderedWalkOverATextOrder => stated_rows(ordered_walk(ColumnIndex(3)), WALKED),
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

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
