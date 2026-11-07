//! One rule evaluated beside the project: its result, its skipped source and a foreign request.
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use super::batch_test::evaluate_fixture;
use crate::application::cross_file::fixtures::counting_plan;
use crate::application::cross_file::fixtures::crossing;
use crate::application::cross_file::fixtures::frontier_rows;
use crate::application::cross_file::fixtures::prepared;
use crate::application::cross_file::fixtures::todo;
use crate::application::cross_file::fixtures::world;
use crate::domain::crossing::service::partials_for;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::types::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::metadata::interface::RelationRead;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use std::sync::Arc;

pub(super) fn current_project_roots_keep_the_original_two_group_four_cell_result()
-> Result<(), RuleRequestsError> {
    use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
    use ryo_execution::inbound::use_cases::execution::types::EmitDemand;
    use ryo_execution::inbound::use_cases::execution::types::ObservationDemand;
    use ryo_execution::inbound::use_cases::execution::types::Partials;
    use ryo_execution::inbound::use_cases::execution::types::ProjectInput;
    use ryo_execution::inbound::use_cases::execution::types::ProjectTask;
    use ryo_execution::inbound::use_cases::execution::types::RootOutcome;
    use ryo_execution::inbound::use_cases::execution::types::RuleDemand;
    let world = world(counting_plan(1, Vec::new()));
    let mut symbols = Symbols::default();
    let (phase, scope, ordinary) =
        prepared(&world, &frontier_rows(), &mut symbols).expect("actual source-bound frontier");
    let partials = Partials::default();
    let ProjectBatch::Complete(mut done) = phase.evaluate(
        vec![ProjectTask {
            rule: RuleIndex(0),
            demand: RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::Required,
            },
            input: ProjectInput {
                scope: &scope,
                evaluation: &partials,
                publication: &partials,
                ordinary,
                bound: &[],
                arenas: &world.arenas,
            },
        }],
        &mut symbols,
        |_| Ok::<(), ryo_execution::inbound::use_cases::execution::types::OperatingError>(()),
    ) else {
        panic!("the guard admits the declared root")
    };
    let done = done.remove(0).expect("rule");
    let RootOutcome::Complete(table) = done.emit else {
        panic!("complete group root")
    };
    assert_eq!(
        [table.count.0, done.counts.fed.len(), table.cells.len()],
        [2, 2, 4]
    );
    Ok(())
}

pub(super) fn rejected_syntax_blocks_cross_file_complements_without_inventing_partial_diagnostics()
-> Result<(), RuleRequestsError> {
    use ryo_values::domain::runs::types::{SkipReason, Skipped};
    let mut world = world(counting_plan(0, Vec::new()));
    let path = ryo_language::domain::files::types::FilePath("rejected.py".into());
    world.paths = vec![path.clone()];
    world.needs[0].cross.push(RelationRead {
        relation: Relation::Function,
        kinds: vec![],
    });
    let skipped = [Skipped {
        rule: world.ids[0].clone(),
        path,
        reason: SkipReason::SyntaxRejected,
    }];
    let mut input = crossing(&world);
    input.domain.skipped = &skipped;
    let partials = partials_for(&MetadataUseCaseImpl, &input, RuleIndex(0));
    assert!(partials.entries.is_empty(), "no provider partial invented");
    assert_eq!(
        partials.unavailable_files,
        [FileId(0)],
        "skipped source remains unavailable for absence checks"
    );
    Ok(())
}

pub(super) fn project_evaluation_rejects_frontiers_from_another_request()
-> Result<(), RuleRequestsError> {
    use ryo_execution::inbound::use_cases::execution::types::OperatingError;
    let rules = world(counting_plan(1, Vec::new()));
    let mut symbols = Symbols::default();
    let (_phase, _scope, frontier) = prepared(&rules, &frontier_rows(), &mut symbols)
        .expect("frontiers admitted by the original request");
    let engine = ExecutionUseCaseImpl::new(
        Arc::new(ryo_execution::outbound::operators::factory::use_case(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())),
        )),
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
    );
    let next = engine.begin(&rules.program, &rules.files);
    let next_phase = next.project_phase().expect("independent project phase");
    let foreign = next_phase
        .project_scope(
            RuleIndex(0),
            &rules.files,
            &rules.covered,
            &[],
            &ryo_execution::inbound::use_cases::execution::types::InapplicableFiles::default(),
        )
        .expect("independent request over the same program and corpus");
    let result = evaluate_fixture(
        &crossing(&rules),
        &todo(0),
        frontier,
        &mut symbols,
        foreign,
        &next_phase,
    );
    match result {
        Err(error) => assert_eq!(
            error,
            ryo_values::domain::rule_requests::service::failed(
                &rules.ids[0],
                &OperatingError::NotRead
            )
        ),
        Ok(_) => panic!("an unrelated request scope cannot admit the original frontiers"),
    }
    Ok(())
}
