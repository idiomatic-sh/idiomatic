//! Project populations stay separate by rule, payload and reached region.

use super::super::super::fixtures::evaluation::evaluate_fixture;
use crate::application::cross_file::fixtures::ProjectCalls;
use crate::application::cross_file::fixtures::counting_plan;
use crate::application::cross_file::fixtures::frontier_rows;
use crate::application::cross_file::fixtures::todo;
use crate::application::cross_file::fixtures::world_with;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use std::sync::Arc;

use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `many_distinct_project_populations_allocate_only_reached_region_state` scenario.
    ManyDistinctProjectPopulationsAllocateOnlyReachedRegionState,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// The rule requests a scenario's run refuses.
type Failure = RuleRequestsError;

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("many_distinct_project_populations_allocate_only_reached_region_state"),
    word: Word::Succeeds,
    input: || Given::ManyDistinctProjectPopulationsAllocateOnlyReachedRegionState,
    expect: Expect::Value(
        "InvocationStateCounts { populations: InvocationStateCount(0), slots: InvocationStateCount(0), instruction_counters: InvocationStateCount(0), dormant_obligations: InvocationStateCount(0) }|2|1|1|1|true|2|2|2|2|true|2|3|3|3|true|2|4|4|4|true|2|5|5|5|true|2|6|6|6|true|2|7|7|7|true|2|8|8|8|true|8|16",
    ),
}];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::ManyDistinctProjectPopulationsAllocateOnlyReachedRegionState => {
            many_distinct_project_populations_allocate_only_reached_region_state()?
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

fn many_distinct_project_populations_allocate_only_reached_region_state()
-> Result<String, RuleRequestsError> {
    let mut observed: Vec<String> = Vec::new();
    use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, QueryLiteral, Test};
    use std::sync::atomic::Ordering;
    const POPULATIONS: usize = 8;
    let calls = Arc::new(ProjectCalls::default());
    let mut plans = Vec::new();
    for seed in 0..96 {
        let mut plan = counting_plan(1, vec![]);
        if seed != 0 {
            // Separate dormant regions, with a semantically valid literal predicate.
            plan.steps.push(PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::Select {
                    from: PlanInput::Step(StepId(1)),
                    comparisons: vec![PlanComparison {
                        test: Test::Equal,
                        left: Operand::Column(ColumnIndex(1)),
                        right: Operand::Literal(QueryLiteral::Number(
                            ryo_relations::domain::relations::types::Integer(seed),
                        )),
                        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                    }],
                    kept: vec![],
                },
            });
            plan.emit.step = StepId(2);
        }
        plans.push(plan);
    }
    let rules = world_with(plans, calls.clone());
    let phase = rules.request.project_phase().unwrap();
    observed.push(format!("{:?}", phase.fixture_state_counts()));
    let mut symbols = Symbols::default();
    for population in 0..POPULATIONS {
        let mut rows = frontier_rows();
        // Distinct legitimate transport payloads force distinct input populations.
        let repeated = rows[0].1.cells[0];
        rows[0].1.cells.push(repeated);
        rows[0].1.count.0 += 1;
        for _ in 0..population {
            rows[0].1.cells.push(repeated);
            rows[0].1.count.0 += 1;
        }
        let result = evaluate_fixture(
            &rules,
            &todo(0),
            &rows,
            &rules.covered,
            &mut symbols,
            &phase,
        )
        .unwrap();
        observed.push(format!(
            "{:?}",
            result.rendered.as_ref().unwrap().order.len()
        ));
        let state = phase.fixture_state_counts();
        observed.push(format!("{:?}", state.populations.0));
        observed.push(format!("{:?}", state.slots.0));
        observed.push(format!("{:?}", state.instruction_counters.0));
        observed.push(format!("{}", state.dormant_obligations.0 <= population + 1));
    }
    observed.push(format!("{:?}", calls.reductions.load(Ordering::SeqCst)));
    observed.push(format!("{:?}", calls.aggregates.load(Ordering::SeqCst)));
    phase.finish().unwrap();
    Ok(observed.join("|"))
}
