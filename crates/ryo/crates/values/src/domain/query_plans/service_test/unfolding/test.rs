//! The cases of the unfolding of a reach join, on values alone: a reach join unfolded keeps every step at its id and gains its closure
//! after the last; a plan with none is answered as it is; each step is visited once, counted at a depth and at twice it;
//! and the unfolded plan read in order and digested, with a digest function of this test's own.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::plan_parts::types::{
    ColumnIndex, Input as PlanInput, JoinColumn, ReachLookup, Side, StepId, StepScope,
};
use crate::domain::plans::types::{Operation, PlanStep};
use crate::domain::query_bindings::types::RuleBindings;
use crate::domain::query_plans::service::{
    compiled_step, node_digests, reach_unfolded, read_order,
};
use crate::domain::query_plans::types::{Emit, NodeBytes, NodeDigest, Plan};
use ryo_relations::domain::relations::types::Relation;
use std::borrow::Cow;
use std::cell::Cell;

/// What a row asks.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// A plan of four steps whose third is a reach join over the first: the steps at each id before and after, and what
    /// was appended.
    ReachJoin,
    /// The same plan with a reach whose columns are the closure's two, the other way round.
    Projected,
    /// A plan with no reach join.
    NoReach,
    /// A ladder of reach joins, each over the join before it on both sides, at a depth and at twice it.
    Ladder,
    /// The compiled step of each step of the projected plan's unfolded plan.
    CompiledSteps,
    /// The order the unfolded plan's steps are read in, and the digests of two unfolded plans that differ only in the edges
    /// their closures read.
    Digested,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = ();

/// A row.
const fn row(
    name: &'static str,
    input: fn() -> Scenario,
    expected: &'static str,
) -> Case<Input, Value, Failure> {
    Case {
        name: CaseName(name),
        word: Word::Succeeds,
        input,
        expect: Expect::Value(Cow::Borrowed(expected)),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    row(
        "a_reach_join_keeps_every_step_at_its_id_and_reads_the_closure_appended_after_the_last",
        || Scenario::ReachJoin,
        r#"3 of 4 steps as they were; step 2: join of Step(StepId(1)) with Step(StepId(4)), no reach; emits step 3; appended: ["closure of Step(StepId(0)) from column 0 to column 1 in 0 keys (PerFile)"]"#,
    ),
    row(
        "a_reach_whose_columns_are_not_the_closures_own_reads_a_projection_appended_after_the_closure",
        || Scenario::Projected,
        r#"3 of 4 steps as they were; step 2: join of Step(StepId(1)) with Step(StepId(5)), no reach; emits step 3; appended: ["closure of Step(StepId(0)) from column 0 to column 1 in 0 keys (PerFile)", "project of Step(StepId(4)) by [1, 0], repeats Kept (PerFile)"]"#,
    ),
    row(
        "a_plan_with_no_reach_join_is_answered_as_it_is",
        || Scenario::NoReach,
        "lent as it is, equal",
    ),
    row(
        "reach_unfolded_appends_one_closure_a_join_under_a_ladder_of_ten_and_of_twenty_links",
        || Scenario::Ladder,
        "21 steps of 11, 10 closures; 41 steps of 21, 20 closures",
    ),
    row(
        "an_appended_step_belongs_to_the_join_it_was_unfolded_from",
        || Scenario::CompiledSteps,
        "[0, 1, 2, 3, 2, 2, 6]",
    ),
    row(
        "an_unfolded_plan_is_read_closure_first_and_its_join_holds_its_closures_digest",
        || Scenario::Digested,
        "read in the order [0, 1, 4, 2, 3]; 10 steps digested of 10; over other edges: the rows to start from equal, the closures differ, the joins differ; written unfolded: the closure equal, the join equal",
    ),
];

fn step(operation: Operation) -> PlanStep {
    PlanStep {
        written: Default::default(),
        operation,
        scope: StepScope::PerFile,
    }
}

/// A plan of `steps` that emits its last.
fn plan_of(steps: Vec<PlanStep>) -> Plan {
    let last = u32::try_from(steps.len().saturating_sub(1)).unwrap_or(0);
    Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        steps,
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        emit: Emit {
            step: StepId(last),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
    }
}

/// A step that keeps every row of `from`.
fn every_row(from: PlanInput) -> PlanStep {
    step(Operation::Select {
        from,
        comparisons: Vec::new(),
        kept: Vec::new(),
    })
}

/// A reach over edges whose source is their first column and whose destination their second, in one graph, seen through
/// `columns`.
fn reach(columns: &[u32]) -> ReachLookup {
    ReachLookup {
        source: ColumnIndex(0),
        destination: ColumnIndex(1),
        keys: Vec::new(),
        columns: columns.iter().copied().map(ColumnIndex).collect(),
    }
}

/// A join of `left` with `right` on their first columns, with `reach` when given.
fn join(left: PlanInput, right: PlanInput, reach: Option<ReachLookup>) -> PlanStep {
    step(Operation::Join {
        left,
        right,
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![JoinColumn {
            side: Side::Right,
            column: ColumnIndex(1),
        }],
        filter: Vec::new(),
        view: Vec::new(),
        reach,
    })
}

/// The edges (step 0), the rows to start from (step 1), their join over the edges with `reach` (step 2) and a step that
/// reads the join (step 3), with the edges read from `edges`.
fn reaching(edges: Relation, reach: Option<ReachLookup>) -> Plan {
    plan_of(vec![
        every_row(PlanInput::Relation(edges)),
        every_row(PlanInput::Relation(Relation::Function)),
        join(
            PlanInput::Step(StepId(1)),
            PlanInput::Step(StepId(0)),
            reach,
        ),
        every_row(PlanInput::Step(StepId(2))),
    ])
}

/// An operation in a few words: what the rows below say of a step.
fn said(operation: &Operation) -> String {
    match operation {
        Operation::Join {
            left, right, reach, ..
        } => format!(
            "join of {left:?} with {right:?}, {}",
            if reach.is_some() {
                "a reach"
            } else {
                "no reach"
            }
        ),
        Operation::Closure {
            edges,
            keys,
            source,
            destination,
        } => format!(
            "closure of {edges:?} from column {} to column {} in {} keys",
            source.0,
            destination.0,
            keys.len()
        ),
        Operation::Project {
            from,
            columns,
            distinct,
        } => format!(
            "project of {from:?} by {:?}, repeats {distinct:?}",
            columns.iter().map(|column| column.0).collect::<Vec<_>>()
        ),
        other => format!("{:?}", crate::domain::plans::service::kind_of(other)),
    }
}

/// How many of the compiled plan's steps the unfolded plan holds unchanged at their ids, what its join says, and what was
/// appended.
fn unfolded_said(compiled: &Plan) -> String {
    let unfolded = reach_unfolded(compiled);
    let same = compiled
        .steps
        .iter()
        .zip(&unfolded.steps)
        .filter(|(before, after)| before == after)
        .count();
    let appended: Vec<String> = unfolded
        .steps
        .iter()
        .skip(compiled.steps.len())
        .map(|step| format!("{} ({:?})", said(&step.operation), step.scope))
        .collect();
    format!(
        "{same} of {} steps as they were; step 2: {}; emits step {}; appended: {appended:?}",
        compiled.steps.len(),
        unfolded
            .steps
            .get(2)
            .map_or_else(String::new, |step| said(&step.operation)),
        unfolded.emit.step.0
    )
}

/// The steps a ladder of `links` reach joins unfolds to: each join reads the step before it on both sides, so a walk that
/// followed every input would append two to the power of the links.
fn ladder(links: u32) -> String {
    let mut steps = vec![every_row(PlanInput::Relation(Relation::Function))];
    for link in 0..links {
        let below = PlanInput::Step(StepId(link));
        steps.push(join(below, below, Some(reach(&[0, 1]))));
    }
    let compiled = plan_of(steps);
    let unfolded = reach_unfolded(&compiled);
    let closures = unfolded
        .steps
        .iter()
        .filter(|step| matches!(step.operation, Operation::Closure { .. }))
        .count();
    format!(
        "{} steps of {}, {closures} closures",
        unfolded.steps.len(),
        compiled.steps.len()
    )
}

/// A digest function of this test's own: the 128-bit FNV-1a fold of the bytes given. It counts the steps it was asked for.
fn folded(asked: &Cell<usize>, bytes: NodeBytes<'_>) -> NodeDigest {
    asked.set(asked.get().saturating_add(1));
    let mut hash: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    for byte in bytes.0 {
        hash = (hash ^ u128::from(*byte)).wrapping_mul(0x0100_0000_0000_0000_0000_013b);
    }
    NodeDigest(hash.to_le_bytes())
}

/// The read order of the unfolded reach plan; then its digests beside those of the same plan over other edges, and beside
/// the closure written as a step of its own before the join in a plan that was never folded.
fn digested() -> String {
    let asked = Cell::new(0);
    let digests =
        |plan: &Plan| node_digests(plan, RuleBindings(&[]), &|bytes| folded(&asked, bytes)).0;
    let (one, other) = (
        reaching(Relation::Function, Some(reach(&[0, 1]))),
        reaching(Relation::ModuleIdentity, Some(reach(&[0, 1]))),
    );
    let (one, other) = (reach_unfolded(&one), reach_unfolded(&other));
    let order: Vec<u32> = read_order(&one).iter().map(|step| step.0).collect();
    let (first, second) = (digests(&one), digests(&other));
    let steps_asked = asked.get();
    // The same query as the compiler wrote it before it folded the closure: the closure a step before the join.
    let written = plan_of(vec![
        every_row(PlanInput::Relation(Relation::Function)),
        every_row(PlanInput::Relation(Relation::Function)),
        step(Operation::Closure {
            edges: PlanInput::Step(StepId(0)),
            keys: Vec::new(),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        }),
        join(PlanInput::Step(StepId(1)), PlanInput::Step(StepId(2)), None),
    ]);
    let third = digests(&written);
    let same = |left: Option<&NodeDigest>, right: Option<&NodeDigest>| {
        if left == right { "equal" } else { "differ" }
    };
    format!(
        "read in the order {order:?}; {steps_asked} steps digested of {}; over other edges: the rows to start from {}, the \
         closures {}, the joins {}; written unfolded: the closure {}, the join {}",
        one.steps.len().saturating_add(other.steps.len()),
        same(first.get(1), second.get(1)),
        same(first.get(4), second.get(4)),
        same(first.get(2), second.get(2)),
        same(first.get(4), third.get(2)),
        same(first.get(2), third.get(3)),
    )
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(Cow::Owned(match scenario {
        Scenario::ReachJoin => unfolded_said(&reaching(Relation::Function, Some(reach(&[0, 1])))),
        Scenario::Projected => unfolded_said(&reaching(Relation::Function, Some(reach(&[1, 0])))),
        Scenario::NoReach => {
            let compiled = reaching(Relation::Function, None);
            let unfolded = reach_unfolded(&compiled);
            format!(
                "{}, {}",
                match unfolded {
                    Cow::Borrowed(_) => "lent as it is",
                    Cow::Owned(_) => "a plan of its own",
                },
                if *unfolded == compiled {
                    "equal"
                } else {
                    "changed"
                }
            )
        }
        Scenario::Ladder => format!("{}; {}", ladder(10), ladder(20)),
        Scenario::CompiledSteps => {
            let compiled = reaching(Relation::Function, Some(reach(&[1, 0])));
            let unfolded = reach_unfolded(&compiled);
            let steps: Vec<u32> = (0..=unfolded.steps.len())
                .map(|place| StepId(u32::try_from(place).unwrap_or(u32::MAX)))
                .map(|step| compiled_step(&compiled, step).0)
                .collect();
            format!("{steps:?}")
        }
        Scenario::Digested => digested(),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
