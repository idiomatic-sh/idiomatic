//! A set of project roots prepares once and delivers in order.
use super::batch_test::assert_count_findings;
use super::batch_test::completed_batch;
use super::batch_test::sealed_fixture_batch;
use crate::application::cross_file::fixtures::ProjectCalls;
use crate::application::cross_file::fixtures::counting_plan;
use crate::application::cross_file::fixtures::crossing;
use crate::application::cross_file::fixtures::frontier_rows;
use crate::application::cross_file::fixtures::prepared_in;
use crate::application::cross_file::fixtures::service;
use crate::application::cross_file::fixtures::todo;
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
-> Result<(), RuleRequestsError> {
    use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
    use std::sync::atomic::Ordering;
    for order in [[0, 1, 0], [1, 0, 1]] {
        let calls = Arc::new(ProjectCalls::default());
        let plan = counting_plan(1, vec![]);
        let rules = world_with(vec![plan.clone(), plan], calls.clone());
        let phase = rules.request.project_phase().unwrap();
        let mut symbols = Symbols::default();
        let mut scopes = Vec::new();
        let mut frontiers = Vec::new();
        let mut todos = Vec::new();
        for rule in order {
            let (scope, frontier) = prepared_in(
                &rules,
                &phase,
                RuleIndex(rule),
                &rules.covered,
                &frontier_rows(),
                &mut symbols,
            )
            .unwrap();
            scopes.push(scope);
            frontiers.push(frontier);
            todos.push(Todo {
                rule: RuleIndex(rule),
                id: rules.ids[rule].clone(),
            });
        }
        let tasks = sealed_fixture_batch(&todos, scopes, frontiers);
        let results = completed_batch(service().evaluate_batch(
            &crossing(&rules),
            tasks,
            &phase,
            &mut symbols,
            |_| Ok(()),
        ));
        assert_eq!(results.len(), todos.len());
        let mut previous = Vec::new();
        for (todo, result) in todos.iter().zip(results) {
            let result = result.unwrap().crossed;
            assert!(result.unknowns.is_empty());
            let rendered = result.rendered.unwrap();
            assert_count_findings(&rendered, todo.rule, &todo.id, &[2, 1], &[1, 0]);
            previous.push(rendered);
        }
        let counts = phase.readiness_counts();
        assert_eq!(
            counts.preparations.0, 1,
            "one canonical root, including duplicate logical roots"
        );
        assert_eq!(counts.evaluations.0, 1);
        assert_eq!(calls.reductions.load(Ordering::SeqCst), 1);
        assert_eq!(calls.aggregates.load(Ordering::SeqCst), 2);
        assert_eq!(phase.state_counts().slots.0, 1);
        let before_replay = phase.readiness_counts();
        let mut again_scopes = Vec::new();
        let mut again_frontiers = Vec::new();
        for todo in &todos {
            let (scope, frontier) = prepared_in(
                &rules,
                &phase,
                todo.rule,
                &rules.covered,
                &frontier_rows(),
                &mut symbols,
            )
            .unwrap();
            again_scopes.push(scope);
            again_frontiers.push(frontier);
        }
        let again = sealed_fixture_batch(&todos, again_scopes, again_frontiers);
        let repeated = completed_batch(service().evaluate_batch(
            &crossing(&rules),
            again,
            &phase,
            &mut symbols,
            |_| Ok(()),
        ));
        assert_eq!(repeated.len(), previous.len());
        for (result, previous) in repeated.into_iter().zip(&previous) {
            assert_eq!(result.unwrap().crossed.rendered.as_ref(), Some(previous));
        }
        assert_eq!(
            phase.readiness_counts(),
            before_replay,
            "delivery adds no hidden fallback polls"
        );
        assert_eq!(calls.reductions.load(Ordering::SeqCst), 1);
        assert_eq!(calls.aggregates.load(Ordering::SeqCst), 2);
        phase.finish().unwrap();
    }
    Ok(())
}

pub(super) fn project_root_set_preserves_distinct_actual_population_policy_in_both_orders()
-> Result<(), RuleRequestsError> {
    use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
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
        let mut scopes = Vec::new();
        let mut frontiers = Vec::new();
        let mut todos = Vec::new();
        for rule in order {
            let source = if rule == 0 { &rows[..1] } else { &rows[..] };
            let (scope, frontier) = prepared_in(
                &rules,
                &phase,
                RuleIndex(rule),
                &rules.covered,
                source,
                &mut symbols,
            )
            .unwrap();
            scopes.push(scope);
            frontiers.push(frontier);
            todos.push(Todo {
                rule: RuleIndex(rule),
                id: rules.ids[rule].clone(),
            });
        }
        let tasks = sealed_fixture_batch(&todos, scopes, frontiers);
        let results = completed_batch(service().evaluate_batch(
            &crossing(&rules),
            tasks,
            &phase,
            &mut symbols,
            |_| Ok(()),
        ));
        assert_eq!(results.len(), order.len());
        for (rule, result) in order.into_iter().zip(results) {
            let result = result.unwrap().crossed;
            // Rule A skipped the second file (its syntax rejected): that file is skipped, never undecided, and rule B reads it.
            assert!(
                result.unknowns.is_empty(),
                "a skipped file is skipped, not unknown: {:?}",
                result.unknowns
            );
            let rendered = result.rendered.unwrap();
            assert_count_findings(
                &rendered,
                RuleIndex(rule),
                &rules.ids[rule],
                if rule == 0 { &[2] } else { &[2, 1] },
                if rule == 0 { &[0] } else { &[1, 0] },
            );
        }
        assert_eq!(phase.readiness_counts().evaluations.0, 2);
        assert_eq!(phase.state_counts().populations.0, 2);
        assert_eq!(calls.reductions.load(Ordering::SeqCst), 2);
        assert_eq!(calls.aggregates.load(Ordering::SeqCst), 3);
        phase.finish().unwrap();
    }
    Ok(())
}
