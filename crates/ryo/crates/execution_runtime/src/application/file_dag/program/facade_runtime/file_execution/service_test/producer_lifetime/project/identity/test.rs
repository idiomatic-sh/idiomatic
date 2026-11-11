//! One project phase execution serves every rule of the request.
use super::super::super::super::super::*;
use super::super::fixtures::{PhaseExit, PreparedCase, prepare_case};
use crate::application::file_dag::frontiers::fixtures::prepared_frontiers;
use crate::application::file_dag::frontiers::types::FrontierInput;
use crate::application::file_dag::frontiers::types::FrontierSource;
use crate::application::file_dag::program::facade_runtime::service::ProjectFrontiers;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::{
    completion::types::CoverageState, coverage::types::Partials, table_model::types::Frontier,
};
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{FileId, Integer, Relation, Value as Cell},
};
use ryo_values::domain::{
    plan_parts::types::Input as PlanInput,
    rows::types::{RowCount, Table},
};
use std::sync::Arc;

/// Which files a rule has no frame for is that rule's own evidence: two rules of one project phase
/// share its one execution although their required partitions differ in it.
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

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    ProjectRulesWithDifferentInapplicableFilesShareOnePhaseExecution,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("project_rules_with_different_inapplicable_files_share_one_phase_execution"),
    word: Word::Succeeds,
    input: || Scenario::ProjectRulesWithDifferentInapplicableFilesShareOnePhaseExecution,
    expect: Expect::Value(()),
}];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::ProjectRulesWithDifferentInapplicableFilesShareOnePhaseExecution => {
            project_rules_with_different_inapplicable_files_share_one_phase_execution();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
