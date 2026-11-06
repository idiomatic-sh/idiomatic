//! Ordinary project rules share one preparation and refuse another generation.
use super::batch_test::assert_count_findings;
use super::batch_test::evaluate_fixture;
use crate::application::cross_file::fixtures::ProjectCalls;
use crate::application::cross_file::fixtures::counting_plan;
use crate::application::cross_file::fixtures::crossing;
use crate::application::cross_file::fixtures::frontier_rows;
use crate::application::cross_file::fixtures::prepared_in;
use crate::application::cross_file::fixtures::todo;
use crate::application::cross_file::fixtures::world_with;
use crate::domain::answering::types::Todo;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::metadata::interface::Covered;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use std::sync::Arc;

pub(super) fn ordinary_project_rules_share_one_preparation_and_reject_another_generation()
-> Result<(), RuleRequestsError> {
    use ryo_execution::inbound::use_cases::execution::types::OperatingError;
    use std::sync::atomic::Ordering;
    for order in [[0, 1], [1, 0]] {
        let calls = Arc::new(ProjectCalls::default());
        let plan = counting_plan(1, Vec::new());
        let mut rules = world_with(vec![plan.clone(), plan], calls.clone());
        let phase = rules.request.project_phase().unwrap();
        let mut symbols = Symbols::default();
        let mut first_rendered = None;
        let mut first_payload = None;
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
            let result = evaluate_fixture(
                &crossing(&rules),
                &Todo {
                    rule: RuleIndex(rule),
                    id: rules.ids[rule].clone(),
                },
                frontier,
                &mut symbols,
                scope,
                &phase,
            )
            .unwrap();
            let rendered = result.rendered.expect("actual core project rendering");
            assert_eq!(result.fed.len(), 2);
            let payload = assert_count_findings(
                &rendered,
                RuleIndex(rule),
                &rules.ids[rule],
                &[2, 1],
                &[1, 0],
            );
            if let Some(first) = &first_payload {
                assert_eq!(&payload, first);
            } else {
                first_payload = Some(payload);
            }
            // Rendering carries the actual logical rule id even when its physical
            // publication is shared. Keep rule 0's full snapshot for its fresh request.
            if rule == 0 {
                first_rendered = Some(rendered);
            }
            assert_eq!(
                calls.reductions.load(Ordering::SeqCst),
                1,
                "one real grouping setup across both logical roots"
            );
            assert_eq!(
                calls.aggregates.load(Ordering::SeqCst),
                2,
                "the two authored node groups aggregate once each"
            );
        }
        // A changed physical coverage declaration cannot consume a cached publication.
        let changed = [Covered {
            file: FileId(0),
            rules: vec![RuleIndex(0)],
            partial: vec![],
        }];
        let different = phase
            .project_scope(
                RuleIndex(1),
                &rules.files,
                &changed,
                &[],
                &ryo_execution::inbound::use_cases::execution::types::InapplicableFiles::default(),
            )
            .unwrap();
        let (_, old_frontier) = prepared_in(
            &rules,
            &phase,
            RuleIndex(1),
            &rules.covered,
            &frontier_rows(),
            &mut symbols,
        )
        .unwrap();
        let result = evaluate_fixture(
            &crossing(&rules),
            &Todo {
                rule: RuleIndex(1),
                id: rules.ids[1].clone(),
            },
            old_frontier,
            &mut symbols,
            different,
            &phase,
        );
        assert!(
            matches!(result, Err(error) if error == ryo_values::domain::rule_requests::service::failed(&rules.ids[1], &OperatingError::NotRead))
        );
        let (changed_scope, changed_frontier) = prepared_in(
            &rules,
            &phase,
            RuleIndex(1),
            &changed,
            &frontier_rows(),
            &mut symbols,
        )
        .unwrap();
        let result = evaluate_fixture(
            &crossing(&rules),
            &Todo {
                rule: RuleIndex(1),
                id: rules.ids[1].clone(),
            },
            changed_frontier,
            &mut symbols,
            changed_scope,
            &phase,
        );
        assert!(
            matches!(result, Err(error) if error == ryo_values::domain::rule_requests::service::failed(
                &rules.ids[1], &OperatingError::WrongOperation,
            )),
            "one interpreter cannot replay a different declared physical partition"
        );
        assert_eq!(calls.reductions.load(Ordering::SeqCst), 1);
        assert_eq!(calls.aggregates.load(Ordering::SeqCst), 2);
        let (_, foreign) = prepared_in(
            &rules,
            &phase,
            RuleIndex(0),
            &rules.covered,
            &frontier_rows(),
            &mut symbols,
        )
        .unwrap();
        phase.finish().unwrap();

        let engine = ExecutionUseCaseImpl::new(
            calls.clone(),
            Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
        );
        rules.request = engine.begin(&rules.program, &rules.files);
        let phase = rules.request.project_phase().unwrap();
        let scope = phase
            .project_scope(
                RuleIndex(0),
                &rules.files,
                &rules.covered,
                &[],
                &ryo_execution::inbound::use_cases::execution::types::InapplicableFiles::default(),
            )
            .unwrap();
        let failed = evaluate_fixture(
            &crossing(&rules),
            &todo(0),
            foreign,
            &mut symbols,
            scope,
            &phase,
        );
        assert!(
            matches!(failed, Err(error) if error == ryo_values::domain::rule_requests::service::failed(&rules.ids[0], &OperatingError::NotRead))
        );
        assert_eq!(calls.reductions.load(Ordering::SeqCst), 1);
        let (scope, frontier) = prepared_in(
            &rules,
            &phase,
            RuleIndex(0),
            &rules.covered,
            &frontier_rows(),
            &mut symbols,
        )
        .unwrap();
        let result = evaluate_fixture(
            &crossing(&rules),
            &todo(0),
            frontier,
            &mut symbols,
            scope,
            &phase,
        )
        .unwrap();
        assert_eq!(result.rendered, first_rendered);
        assert_eq!(
            calls.reductions.load(Ordering::SeqCst),
            2,
            "a new request requires its own grouping payload"
        );
        assert_eq!(calls.aggregates.load(Ordering::SeqCst), 4);
        phase.finish().unwrap();
    }
    Ok(())
}
