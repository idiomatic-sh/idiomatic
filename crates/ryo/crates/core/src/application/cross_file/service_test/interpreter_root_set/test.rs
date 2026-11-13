//! A set of project roots prepares once and delivers in order.

use super::super::super::fixtures::evaluation::completed_batch;
use super::super::super::fixtures::evaluation::count_findings;
use super::super::super::fixtures::evaluation::evaluate_rules;
use crate::application::cross_file::fixtures::ProjectCalls;
use crate::application::cross_file::fixtures::counting_plan;
use crate::application::cross_file::fixtures::frontier_rows;
use crate::application::cross_file::fixtures::world_with;
use crate::application::cross_file::fixtures::world_with_applicability;
use crate::domain::answering::types::Todo;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::metadata::interface::Covered;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use std::sync::Arc;

use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `project_root_set_prepares_shared_publications_once_before_ordered_delivery` scenario.
    ProjectRootSetPreparesSharedPublicationsOnceBeforeOrderedDelivery,
    /// The `project_root_set_preserves_distinct_actual_population_policy_in_both_orders` scenario.
    ProjectRootSetPreservesDistinctActualPopulationPolicyInBothOrders,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// The rule requests a scenario's run refuses.
type Failure = RuleRequestsError;

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "project_root_set_prepares_shared_publications_once_before_ordered_delivery",
        ),
        word: Word::Succeeds,
        input: || Given::ProjectRootSetPreparesSharedPublicationsOnceBeforeOrderedDelivery,
        expect: Expect::Value(
            "3|true|rule,2,0,-,-,rule,1,0,-,- rulerule [5, 10] 0:0:0:1,0:0:0:0|true|rule-1,2,0,-,-,rule-1,1,0,-,- rule-1rule-1 [5, 10] 1:0:0:1,1:0:0:0|true|rule,2,0,-,-,rule,1,0,-,- rulerule [5, 10] 0:0:0:1,0:0:0:0|1|1|1|2|1|3|true|true|true|true|1|2|3|true|rule-1,2,0,-,-,rule-1,1,0,-,- rule-1rule-1 [5, 10] 1:0:0:1,1:0:0:0|true|rule,2,0,-,-,rule,1,0,-,- rulerule [5, 10] 0:0:0:1,0:0:0:0|true|rule-1,2,0,-,-,rule-1,1,0,-,- rule-1rule-1 [5, 10] 1:0:0:1,1:0:0:0|1|1|1|2|1|3|true|true|true|true|1|2",
        ),
    },
    Case {
        name: CaseName(
            "project_root_set_preserves_distinct_actual_population_policy_in_both_orders",
        ),
        word: Word::Succeeds,
        input: || Given::ProjectRootSetPreservesDistinctActualPopulationPolicyInBothOrders,
        expect: Expect::Value(
            "2|true|rule,2,0,-,- rule [5] 0:0:0:0|true|rule-1,2,0,-,-,rule-1,1,0,-,- rule-1rule-1 [5, 10] 1:0:0:1,1:0:0:0|2|2|2|3|2|true|rule-1,2,0,-,-,rule-1,1,0,-,- rule-1rule-1 [5, 10] 1:0:0:1,1:0:0:0|true|rule,2,0,-,- rule [5] 0:0:0:0|2|2|2|3",
        ),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::ProjectRootSetPreparesSharedPublicationsOnceBeforeOrderedDelivery => {
            project_root_set_prepares_shared_publications_once_before_ordered_delivery()?
        }
        Given::ProjectRootSetPreservesDistinctActualPopulationPolicyInBothOrders => {
            project_root_set_preserves_distinct_actual_population_policy_in_both_orders()?
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

fn project_root_set_prepares_shared_publications_once_before_ordered_delivery()
-> Result<String, RuleRequestsError> {
    let mut observed: Vec<String> = Vec::new();
    use std::sync::atomic::Ordering;
    for order in [[0, 1, 0], [1, 0, 1]] {
        let calls = Arc::new(ProjectCalls::default());
        let plan = counting_plan(1, vec![]);
        let rules = world_with(vec![plan.clone(), plan], calls.clone());
        let phase = rules.request.project_phase().unwrap();
        let mut symbols = Symbols::default();
        let todos = order
            .iter()
            .map(|&rule| Todo {
                rule: RuleIndex(rule),
                id: rules.ids[rule].clone(),
            })
            .collect::<Vec<_>>();
        let rows = frontier_rows();
        let kept: [(RuleIndex, &[_]); 2] = [(RuleIndex(0), &rows), (RuleIndex(1), &rows)];
        let results = completed_batch(evaluate_rules(
            &rules,
            &phase,
            &todos,
            &kept,
            &mut symbols,
            |_| Ok(()),
        ));
        observed.push(format!("{:?}", results.len()));
        let mut previous = Vec::new();
        for result in results {
            let result = result.unwrap().crossed;
            observed.push(format!("{}", result.unknowns.is_empty()));
            let rendered = result.rendered.unwrap();
            observed.push(count_findings(&rendered).0);
            previous.push(rendered);
        }
        let counts = phase.fixture_readiness_counts();
        observed.push(format!("{:?}", counts.preparations.0));
        observed.push(format!("{:?}", counts.evaluations.0));
        observed.push(format!("{:?}", calls.reductions.load(Ordering::SeqCst)));
        observed.push(format!("{:?}", calls.aggregates.load(Ordering::SeqCst)));
        observed.push(format!("{:?}", phase.fixture_state_counts().slots.0));
        let before_replay = phase.fixture_readiness_counts();
        let repeated = completed_batch(evaluate_rules(
            &rules,
            &phase,
            &todos,
            &kept,
            &mut symbols,
            |_| Ok(()),
        ));
        observed.push(format!("{:?}", repeated.len()));
        for (result, previous) in repeated.into_iter().zip(&previous) {
            observed.push(format!(
                "{:?}",
                result.unwrap().crossed.rendered.as_ref() == Some(previous)
            ));
        }
        observed.push(format!(
            "{:?}",
            phase.fixture_readiness_counts() == before_replay
        ));
        observed.push(format!("{:?}", calls.reductions.load(Ordering::SeqCst)));
        observed.push(format!("{:?}", calls.aggregates.load(Ordering::SeqCst)));
        phase.finish().unwrap();
    }
    Ok(observed.join("|"))
}

fn project_root_set_preserves_distinct_actual_population_policy_in_both_orders()
-> Result<String, RuleRequestsError> {
    let mut observed: Vec<String> = Vec::new();
    use ryo_values::domain::runs::types::{RuleApplicability, SkipReason, Skipped};
    use std::sync::atomic::Ordering;
    for order in [[0, 1], [1, 0]] {
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
        let todos = order
            .iter()
            .map(|&rule| Todo {
                rule: RuleIndex(rule),
                id: rules.ids[rule].clone(),
            })
            .collect::<Vec<_>>();
        let kept: [(RuleIndex, &[_]); 2] = [(RuleIndex(0), &rows[..1]), (RuleIndex(1), &rows[..])];
        let results = completed_batch(evaluate_rules(
            &rules,
            &phase,
            &todos,
            &kept,
            &mut symbols,
            |_| Ok(()),
        ));
        observed.push(format!("{:?}", results.len()));
        for result in results {
            let result = result.unwrap().crossed;
            // Rule A skipped the second file (its syntax rejected): that file is skipped, never undecided, and rule B reads it.
            observed.push(format!("{}", result.unknowns.is_empty()));
            let rendered = result.rendered.unwrap();
            observed.push(count_findings(&rendered).0);
        }
        observed.push(format!(
            "{:?}",
            phase.fixture_readiness_counts().evaluations.0
        ));
        observed.push(format!("{:?}", phase.fixture_state_counts().populations.0));
        observed.push(format!("{:?}", calls.reductions.load(Ordering::SeqCst)));
        observed.push(format!("{:?}", calls.aggregates.load(Ordering::SeqCst)));
        phase.finish().unwrap();
    }
    Ok(observed.join("|"))
}
