//! Project populations stay separate by rule, payload and reached region.
use crate::application::cross_file::fixtures::counting_plan;
use crate::application::cross_file::fixtures::crossing;
use crate::application::cross_file::fixtures::evaluation::count_findings;
use crate::application::cross_file::fixtures::evaluation::evaluate_fixture;
use crate::application::cross_file::fixtures::frontier_rows;
use crate::application::cross_file::fixtures::project_calls::ProjectCalls;
use crate::application::cross_file::fixtures::todo;
use crate::application::cross_file::fixtures::world_with;
use crate::application::cross_file::fixtures::world_with_applicability;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::answering::types::Todo;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::metadata::interface::Covered;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use std::sync::Arc;

fn ordinary_phase_keeps_valid_rule_populations_and_partiality_separate()
-> Result<String, RuleRequestsError> {
    let mut seen: Vec<String> = Vec::new();
    use ryo_values::domain::runs::types::{RuleApplicability, SkipReason, Skipped};
    use std::sync::atomic::Ordering;
    for order in [[0, 1, 0, 1], [1, 0, 1, 0]] {
        let calls = Arc::new(ProjectCalls::default());
        let plan = counting_plan(1, vec![]);
        let mut rules = world_with_applicability(
            vec![plan.clone(), plan],
            calls.clone(),
            &[RuleApplicability::AcceptedSyntax, RuleApplicability::Always],
        );
        rules.skipped.push(Skipped {
            rule: rules.ids[0].clone(),
            path: rules.paths[1].clone(),
            reason: SkipReason::SyntaxRejected,
        });
        rules.covered = vec![
            Covered {
                file: FileId(0),
                rules: vec![RuleIndex(0), RuleIndex(1)],
                partial: vec![],
            },
            Covered {
                file: FileId(1),
                rules: vec![RuleIndex(1)],
                partial: vec![],
            },
        ];
        let phase = rules.request.project_phase().unwrap();
        let mut symbols = Symbols::default();
        let rows = frontier_rows();
        let mut results = vec![None, None];
        for (at, rule) in order.into_iter().enumerate() {
            // Both scopes receive the same complete request table. Only A has
            // positive skip provenance; B must retain the second file's rows.
            let source = if rule == 0 { &rows[..1] } else { &rows[..] };
            let declared = Todo {
                rule: RuleIndex(rule),
                id: rules.ids[rule].clone(),
            };
            let done = evaluate_fixture(
                &rules,
                &declared,
                source,
                &rules.covered,
                &mut symbols,
                &phase,
            )
            .unwrap();
            let rendered = done
                .rendered
                .expect("valid population renders through the actual core route");
            seen.push(count_findings(&rendered).0);
            // Rule A skipped the second file (its syntax rejected): that file is skipped, never undecided, and rule B reads it.
            seen.push(format!("{:?}", done.unknowns.is_empty()));
            if let Some(previous) = &results[rule] {
                seen.push(format!("{:?}", (&rendered) == (previous)));
            } else {
                results[rule] = Some(rendered);
            }
            if at >= 1 {
                seen.push(format!("{:?}", calls.reductions.load(Ordering::SeqCst)));
                seen.push(format!("{:?}", calls.aggregates.load(Ordering::SeqCst)));
            }
        }
        phase.finish().unwrap();
    }
    Ok(seen.join("|"))
}

fn equal_policies_do_not_alias_distinct_admitted_frontier_payloads()
-> Result<String, RuleRequestsError> {
    let mut seen: Vec<String> = Vec::new();
    use ryo_relations::domain::relations::{service::node_id, types::Position};
    use std::sync::atomic::Ordering;
    for order in [[0, 1, 0], [1, 0, 1]] {
        let calls = Arc::new(ProjectCalls::default());
        let plan = counting_plan(1, vec![]);
        let rules = world_with(vec![plan.clone(), plan], calls.clone());
        let phase = rules.request.project_phase().unwrap();
        let mut symbols = Symbols::default();
        let original = frontier_rows();
        let mut changed = original.clone();
        for cell in &mut changed[0].1.cells {
            *cell = Cell::Node(node_id(FileId(0), Position(9)));
        }
        let mut results = vec![None, None];
        for (at, rule) in order.into_iter().enumerate() {
            let source = if rule == 0 { &original } else { &changed };
            use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
            let crossing = crossing(&rules);
            let declared = Todo {
                rule: RuleIndex(rule),
                id: rules.ids[rule].clone(),
            };
            let outcome = crate::application::cross_file::fixtures::evaluation::evaluate_outcome(
                &rules,
                &phase,
                RuleIndex(rule),
                source,
                &rules.covered,
                || {
                    crate::application::cross_file::service::phase_partials(
                        &crossing.program,
                        &crossing.domain,
                        &declared,
                    )
                },
                &mut symbols,
            );
            let mut outcomes = vec![outcome];
            seen.push(format!("{:?}", outcomes.len()));
            let table = match outcomes
                .pop()
                .unwrap()
                .expect("the occurrence is evaluated")
                .emit
            {
                RootOutcome::Complete(table) | RootOutcome::Partial { table, .. } => table,
                RootOutcome::Failed(error) | RootOutcome::Unknown(error) => {
                    seen.push(format!("refused: {error}"));
                    return Ok(seen.join("|"));
                }
                RootOutcome::Skipped(_) | RootOutcome::NoQuery | RootOutcome::Deferred { .. } => {
                    seen.push("absent".to_owned());
                    return Ok(seen.join("|"));
                }
            };
            let first = if rule == 0 { 1 } else { 9 };
            seen.push(format!(
                "{:?}",
                (table.cells)
                    == ([
                        Cell::Node(node_id(FileId(0), Position(first))),
                        Cell::Number(ryo_relations::domain::relations::types::Integer(2)),
                        Cell::Node(node_id(FileId(1), Position(2))),
                        Cell::Number(ryo_relations::domain::relations::types::Integer(1)),
                    ])
            ));
            let declared = Todo {
                rule: RuleIndex(rule),
                id: rules.ids[rule].clone(),
            };
            let done = evaluate_fixture(
                &rules,
                &declared,
                source,
                &rules.covered,
                &mut symbols,
                &phase,
            )
            .unwrap();
            seen.push(format!("{:?}", done.unknowns.is_empty()));
            let rendered = done.rendered.unwrap();
            seen.push(count_findings(&rendered).0);
            if let Some(previous) = &results[rule] {
                seen.push(format!("{:?}", (&rendered) == (previous)));
            } else {
                results[rule] = Some(rendered);
            }
            if at >= 1 {
                seen.push(format!("{:?}", calls.reductions.load(Ordering::SeqCst)));
                seen.push(format!("{:?}", calls.aggregates.load(Ordering::SeqCst)));
            }
        }
        phase.finish().unwrap();
    }
    Ok(seen.join("|"))
}

fn many_distinct_project_populations_allocate_only_reached_region_state()
-> Result<String, RuleRequestsError> {
    let mut seen: Vec<String> = Vec::new();
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
        seen.push(format!(
            "{:?}",
            result.rendered.as_ref().unwrap().order.len()
        ));
    }
    seen.push(format!(
        "{:?}",
        (calls.reductions.load(Ordering::SeqCst)) == (POPULATIONS)
    ));
    seen.push(format!(
        "{:?}",
        (calls.aggregates.load(Ordering::SeqCst)) == (POPULATIONS * 2)
    ));
    phase.finish().unwrap();
    Ok(seen.join("|"))
}

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `ordinary_phase_keeps_valid_rule_populations_and_partiality_separate` scenario.
    OrdinaryPhaseKeepsValidRulePopulationsAndPartialitySeparate,
    /// The `equal_policies_do_not_alias_distinct_admitted_frontier_payloads` scenario.
    EqualPoliciesDoNotAliasDistinctAdmittedFrontierPayloads,
    /// The `many_distinct_project_populations_allocate_only_reached_region_state` scenario.
    ManyDistinctProjectPopulationsAllocateOnlyReachedRegionState,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// What a scenario's run refuses.
type Failure = RuleRequestsError;

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ordinary_phase_keeps_valid_rule_populations_and_partiality_separate"),
        word: Word::Succeeds,
        input: || Given::OrdinaryPhaseKeepsValidRulePopulationsAndPartialitySeparate,
        expect: Expect::Value(
            "rule,2,0,-,- rule [5] 0:0:0:0|true|rule-1,2,0,-,-,rule-1,1,0,-,- rule-1rule-1 [5, 10] 1:0:0:1,1:0:0:0|true|2|3|rule,2,0,-,- rule [5] 0:0:0:0|true|true|2|3|rule-1,2,0,-,-,rule-1,1,0,-,- rule-1rule-1 [5, 10] 1:0:0:1,1:0:0:0|true|true|2|3|rule-1,2,0,-,-,rule-1,1,0,-,- rule-1rule-1 [5, 10] 1:0:0:1,1:0:0:0|true|rule,2,0,-,- rule [5] 0:0:0:0|true|2|3|rule-1,2,0,-,-,rule-1,1,0,-,- rule-1rule-1 [5, 10] 1:0:0:1,1:0:0:0|true|true|2|3|rule,2,0,-,- rule [5] 0:0:0:0|true|true|2|3",
        ),
    },
    Case {
        name: CaseName("equal_policies_do_not_alias_distinct_admitted_frontier_payloads"),
        word: Word::Succeeds,
        input: || Given::EqualPoliciesDoNotAliasDistinctAdmittedFrontierPayloads,
        expect: Expect::Value(
            "1|true|true|rule,2,0,-,-,rule,1,0,-,- rulerule [5, 10] 0:0:0:1,0:0:0:0|1|true|true|rule-1,2,0,-,-,rule-1,1,0,-,- rule-1rule-1 [5, 10] 1:0:0:1,1:0:0:0|2|4|1|true|true|rule,2,0,-,-,rule,1,0,-,- rulerule [5, 10] 0:0:0:1,0:0:0:0|true|2|4|1|true|true|rule-1,2,0,-,-,rule-1,1,0,-,- rule-1rule-1 [5, 10] 1:0:0:1,1:0:0:0|1|true|true|rule,2,0,-,-,rule,1,0,-,- rulerule [5, 10] 0:0:0:1,0:0:0:0|2|4|1|true|true|rule-1,2,0,-,-,rule-1,1,0,-,- rule-1rule-1 [5, 10] 1:0:0:1,1:0:0:0|true|2|4",
        ),
    },
    Case {
        name: CaseName("many_distinct_project_populations_allocate_only_reached_region_state"),
        word: Word::Succeeds,
        input: || Given::ManyDistinctProjectPopulationsAllocateOnlyReachedRegionState,
        expect: Expect::Value("2|2|2|2|2|2|2|2|true|true"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::OrdinaryPhaseKeepsValidRulePopulationsAndPartialitySeparate => {
            ordinary_phase_keeps_valid_rule_populations_and_partiality_separate()?
        }
        Given::EqualPoliciesDoNotAliasDistinctAdmittedFrontierPayloads => {
            equal_policies_do_not_alias_distinct_admitted_frontier_payloads()?
        }
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
