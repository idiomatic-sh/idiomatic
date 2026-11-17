//! The graph kinds through `plan` and `run` only: what a closure, a closure fused into its join and a group digest answer,
//! and what a seeded closure walks, against the same rows run another way or recorded from main's row meaning.
use super::super::super::fixtures::graph_workloads::defines_of_edges;
use super::super::super::fixtures::{TableInputs, TextResources, emitted};
use super::fixtures::{EDGES, closure, edges, join, left_of, reach, self_matching, stated};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::Input as StepInput;

#[derive(Clone, Copy)]
enum Scenario {
    WholeClosure,
    SeededAsWhole,
    SeededAsWholeWithResidualKey,
    SeededWalksOnlyBound,
    BoundWalkedOnce,
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
            1, 1, 1, 2, 1, 3, 1, 4, 2, 1, 2, 2, 2, 3, 2, 4, 3, 1, 3, 2, 3, 3, 3, 4, 5, 1, 5, 2, 5,
            3, 5, 4,
        ]))),
    },
    Case {
        name: CaseName("a_seeded_closure_answers_what_the_join_over_the_whole_closure_answers"),
        word: Word::Succeeds,
        input: || Scenario::SeededAsWhole,
        expect: Expect::Value(Observation::Same(true)),
    },
    Case {
        name: CaseName(
            "a_seeded_closure_answers_what_the_join_over_the_whole_closure_answers_with_a_residual_key",
        ),
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
        name: CaseName("a_graph_kernel_raises_its_edges_fault_first"),
        word: Word::Succeeds,
        input: || Scenario::EdgesFaultFirst,
        expect: Expect::Value(Observation::Same(true)),
    },
];

/// What the join over the whole closure and the seeded closure answer over the same edges.
fn seeded_as_whole(residual: bool) -> Result<Value, Failure> {
    let mut resources = TextResources::new(&["f"]);
    let table = edges(
        &[
            (1, 2, 0),
            (2, 3, 0),
            (3, 1, 0),
            (1, 4, 1),
            (5, 1, 0),
            (1, 2, 2),
            (6, 7, 0),
        ],
        &resources,
    );
    let inputs = TableInputs::new(&[(EDGES, &table)]);
    let whole = [
        closure(StepInput::Relation(EDGES), PlanInput::Relation(EDGES)),
        join(
            (
                StepInput::Step(ryo_values::domain::plan_parts::types::StepId(0)),
                PlanInput::Node(PlanNodeId(0)),
            ),
            residual,
            None,
        ),
    ];
    let seeded = [join(
        (StepInput::Relation(EDGES), PlanInput::Relation(EDGES)),
        residual,
        Some(reach()),
    )];
    let over_whole = emitted(
        &whole,
        &[PlanNodeId(1)],
        &inputs,
        &mut resources,
        &mut FileBuffers::default(),
    )?;
    let over_seeds = emitted(
        &seeded,
        &[PlanNodeId(0)],
        &inputs,
        &mut resources,
        &mut FileBuffers::default(),
    )?;
    Ok(Observation::Same(
        over_whole == over_seeds && over_whole.first().is_some_and(|rows| rows.count.0 > 0),
    ))
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
    let left = defines_of_edges(&edges(&starts, &resources));
    let inputs = TableInputs::new(&[(EDGES, &table), (Relation::Defines, &left)]);
    let step = left_of(
        join(
            (StepInput::Relation(EDGES), PlanInput::Relation(EDGES)),
            false,
            Some(reach()),
        ),
        (
            StepInput::Relation(Relation::Defines),
            PlanInput::Relation(Relation::Defines),
        ),
    );
    let mut buffers = FileBuffers::default();
    let _rows = emitted(
        &[step],
        &[PlanNodeId(0)],
        &inputs,
        &mut resources,
        &mut buffers,
    )?;
    Ok(Observation::Walked(
        buffers.walk.runs.len(),
        buffers.walk.reached.len(),
    ))
}

/// A closure fused into its join reads its left rows (a select failing on row 1) as they stream and its edges (a select
/// failing on row 4) whole before it walks: it reports the edges' fault, the one their select alone reports, and not the
/// left rows' fault of an earlier row.
fn edges_fault_first() -> Result<Value, Failure> {
    let mut resources = TextResources::new(&["f", "(", "["]);
    let fields = |bad: usize, text: &str| {
        let mut table = edges(
            &[
                (1, 2, 0),
                (2, 3, 0),
                (3, 4, 0),
                (4, 5, 0),
                (5, 6, 0),
                (6, 7, 0),
            ],
            &resources,
        );
        if let Some(field) = table.cells.get_mut(bad * 5 + 3) {
            *field = resources.symbol(text);
        }
        table
    };
    let (left_rows, edge_rows) = (defines_of_edges(&fields(1, "(")), fields(4, "["));
    let inputs = TableInputs::new(&[(EDGES, &edge_rows), (Relation::Defines, &left_rows)]);
    let node = |at| {
        (
            StepInput::Step(ryo_values::domain::plan_parts::types::StepId(at)),
            PlanInput::Node(PlanNodeId(at)),
        )
    };
    let seeded = left_of(join(node(1), false, Some(reach())), node(0));
    let (left, edges) = (self_matching(Relation::Defines), self_matching(EDGES));
    let mut run = |steps: &[Step]| {
        let last = PlanNodeId(u32::try_from(steps.len() - 1).unwrap_or(u32::MAX));
        emitted(
            steps,
            &[last],
            &inputs,
            &mut resources,
            &mut FileBuffers::default(),
        )
    };
    let both = run(&[left.clone(), edges.clone(), seeded]);
    let Err(first) = run(&[edges]) else {
        return Ok(Observation::Same(false));
    };
    let Err(streamed) = run(&[left]) else {
        return Ok(Observation::Same(false));
    };
    Ok(Observation::Same(
        both == Err(first.clone()) && streamed != first,
    ))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::EdgesFaultFirst => edges_fault_first(),
        Scenario::WholeClosure => {
            let mut resources = TextResources::new(&["f"]);
            let table = edges(
                &[
                    (3, 1, 0),
                    (1, 2, 0),
                    (2, 3, 0),
                    (1, 4, 1),
                    (5, 1, 0),
                    (1, 2, 2),
                ],
                &resources,
            );
            let inputs = TableInputs::new(&[(EDGES, &table)]);
            let steps = [closure(
                StepInput::Relation(EDGES),
                PlanInput::Relation(EDGES),
            )];
            let rows = emitted(
                &steps,
                &[PlanNodeId(0)],
                &inputs,
                &mut resources,
                &mut FileBuffers::default(),
            )?;
            Ok(Observation::Rows(
                rows.iter()
                    .flat_map(|table| table.cells.iter().copied().map(stated))
                    .collect(),
            ))
        }
        Scenario::SeededAsWhole => seeded_as_whole(false),
        Scenario::SeededAsWholeWithResidualKey => seeded_as_whole(true),
        Scenario::SeededWalksOnlyBound => walked(1_000, 10, &[3, 700], 1),
        Scenario::BoundWalkedOnce => walked(10, 10, &[4], 100),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
