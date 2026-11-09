//! One project phase execution serves every rule of the request.
use super::super::super::super::*;
use super::super::fixtures::{PhaseExit, PreparedCase, prepare_case};
use crate::application::file_dag::frontiers::fixtures::prepared_frontiers;
use crate::application::file_dag::frontiers::types::FrontierInput;
use crate::application::file_dag::frontiers::types::FrontierSource;
use crate::application::file_dag::program::facade_runtime::types::ProjectFrontiers;
use ryo_execution_domain::domain::{
    completion::types::CoverageState, coverage::types::Partials, table_model::types::Frontier,
};

/// Which files a rule has no frame for is that rule's own evidence: two rules of one project phase
/// share its one execution although their required partitions differ in it.
#[test]
fn project_rules_with_different_inapplicable_files_share_one_phase_execution() {
    let PreparedCase {
        prepared,
        services,
        corpus,
    } = prepare_case(PhaseExit::Complete).expect("prepared case");
    let phase = crate::application::file_dag::program::facade_runtime::service::publication_owner(
        &prepared,
        &services,
        std::sync::Arc::new(crate::outbound::digest::fixtures::fake::DigestFake),
    )
    .expect("publication owner");
    let required = |inapplicable: &[FileId]| {
        crate::application::file_dag::program::facade_runtime::service::required_partitions(
            &prepared,
            RuleIndex(0),
            &corpus,
            &[],
            &[],
            inapplicable,
            &[],
            &services,
        )
        .expect("required partitions")
    };
    let source = PlanInput::Relation(Relation::LanguageTarget);
    let source_root = prepared.program().rules()[0].frontier[0].clone();
    let partials = Partials::default();
    let mut symbols = Symbols::default();
    for rule in 0..2 {
        let required = required(&[FileId(0)][..1 - rule]);
        let table = Table {
            kinds: kinds(Relation::LanguageTarget).to_vec(),
            cells: vec![
                Cell::File(FileId(0)),
                Cell::Number(Integer(2)),
                Cell::Number(Integer(9)),
            ],
            count: RowCount(1),
        };
        crate::application::file_dag::program::facade_runtime::project_execution::open_project(
            &prepared,
            required.clone(),
            ScopePartials::Project {
                evaluation: &partials,
                publication: &partials,
            },
            ProjectFrontiers {
                publication: &phase,
                ordinary: prepared_frontiers(
                    RuleIndex(rule),
                    vec![FrontierSource {
                        input: source,
                        root: source_root.clone(),
                    }]
                    .into(),
                    vec![FrontierInput::Decoded {
                        frontier: Frontier {
                            input: source,
                            table: Arc::new(table),
                        },
                        coverage: CoverageState::Complete,
                    }]
                    .into(),
                    Arc::clone(prepared.program()),
                    required,
                    vec![].into(),
                ),
                bound_inputs: &[],
            },
            &mut symbols,
            &[],
            Arc::clone(&services),
        )
        .unwrap_or_else(|error| panic!("rule {rule} opens the shared phase execution: {error:?}"));
    }
}
