//! Finishing the phase refuses while a scope still shares the publication.
use super::super::super::source_facts::fixtures::ProjectFacts;
use super::super::fixtures::NoFileFrontiers;
use crate::boundary_fixture::{corpus, engine, prepared};
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::PhaseOccurrences;
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
            &ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default(),
        )
        .unwrap();
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let mut symbols = Symbols::default();
    let project_symbols = Symbols::default();
    let partials = Partials::default();
    crate::boundary_fixture::project_root(
        &phase,
        PhaseOccurrences {
            occurrences: &[crate::boundary_fixture::ordinary_occurrence(RuleIndex(0))],
            first: &[Bucket(0)],
            corpus: &corpus,
            covered: &covered,
            skipped: &[],
            inapplicable: &[
                ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(
                ),
            ],
            refused: &[
                ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default(),
            ],
            facts: &NoFileFrontiers,
            project: &project,
            project_symbols: &project_symbols,
            arenas: &[],
        },
        &partials,
        &partials,
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
