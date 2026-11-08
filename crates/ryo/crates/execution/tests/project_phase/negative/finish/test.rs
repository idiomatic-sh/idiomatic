//! Finishing the phase refuses while a scope still shares the publication.
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use super::super::super::source_facts::fixtures::ProjectFacts;
use super::super::fixtures::NoFileFrontiers;
use crate::boundary_fixture::{corpus, demand, engine, prepared, project_root};
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::ProjectInput;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::FrontierDemand;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::FrontierRequest;
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::domain::stored_facts::types::Bucket;

#[test]
fn finish_refuses_while_a_scope_still_shares_the_publication() {
    let engine = engine();
    let prepared = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus);
    let covered = [Covered {
        file: FileId(0),
        rules: vec![RuleIndex(0)],
        partial: vec![],
    }];
    let phase = request.project_phase().unwrap();
    let scope = phase
        .project_scope(
            RuleIndex(0),
            &corpus,
            &covered,
            &[],
            &ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(),
        )
        .unwrap();
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let mut symbols = Symbols::default();
    let project_symbols = Symbols::default();
    let ordinary = request
        .prepare_frontiers(
            FrontierRequest {
                demand: FrontierDemand::Diagnostic,
                scope: &scope,
                rule: RuleIndex(0),
                first: Bucket(0),
                facts: &NoFileFrontiers,
                project: &project,
                project_symbols: &project_symbols,
            },
            &mut symbols,
        )
        .unwrap();
    let partials = Partials::default();
    project_root(
        &phase,
        RuleIndex(0),
        demand(),
        ProjectInput {
            scope: &scope,
            evaluation: &partials,
            publication: &partials,
            ordinary,
            bound: &[],
            arenas: &[],
        },
        &mut symbols,
    )
    .unwrap();
    assert!(
        matches!(phase.finish(), Err(OperatingError::NotRead)),
        "an outstanding scope keeps the publication shared, so finish reports the refusal"
    );
    drop(scope);
    request.finish();
}
