//! The helpers of the `interpreter_root_set` group's scenarios.

use super::super::super::fixtures::cases::assert_count_findings;
use super::super::super::fixtures::cases::completed_batch;
use super::super::super::fixtures::cases::evaluate_rules;
use crate::application::cross_file::fixtures::ProjectCalls;
use crate::application::cross_file::fixtures::counting_plan;
use crate::application::cross_file::fixtures::frontier_rows;
use crate::application::cross_file::fixtures::world_with;
use crate::application::cross_file::fixtures::world_with_applicability;
use crate::domain::answering::types::Todo;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::metadata::interface::Covered;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use std::sync::Arc;

pub(super) fn project_root_set_prepares_shared_publications_once_before_ordered_delivery()
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
        for (todo, result) in todos.iter().zip(results) {
            let result = result.unwrap().crossed;
            observed.push(format!("{}", result.unknowns.is_empty()));
            let rendered = result.rendered.unwrap();
            assert_count_findings(&rendered, todo.rule, &todo.id, &[2, 1], &[1, 0]);
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
            observed.push(format!("{:?}", result.unwrap().crossed.rendered.as_ref()));
        }
        observed.push(format!("{:?}", phase.fixture_readiness_counts()));
        observed.push(format!("{:?}", calls.reductions.load(Ordering::SeqCst)));
        observed.push(format!("{:?}", calls.aggregates.load(Ordering::SeqCst)));
        phase.finish().unwrap();
    }
    Ok(observed.join("|"))
}

pub(super) fn project_root_set_preserves_distinct_actual_population_policy_in_both_orders()
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
        for (rule, result) in order.into_iter().zip(results) {
            let result = result.unwrap().crossed;
            // Rule A skipped the second file (its syntax rejected): that file is skipped, never undecided, and rule B reads it.
            observed.push(format!("{}", result.unknowns.is_empty()));
            let rendered = result.rendered.unwrap();
            assert_count_findings(
                &rendered,
                RuleIndex(rule),
                &rules.ids[rule],
                if rule == 0 { &[2] } else { &[2, 1] },
                if rule == 0 { &[0] } else { &[1, 0] },
            );
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
