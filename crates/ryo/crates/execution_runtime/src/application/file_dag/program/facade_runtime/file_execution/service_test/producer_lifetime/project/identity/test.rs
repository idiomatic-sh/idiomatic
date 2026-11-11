//! One project phase execution serves every rule of the request.
use super::super::fixtures::{PhaseExit, PreparedCase, prepare_case};
use crate::application::file_dag::frontiers::fixtures::prepared_frontiers;
use crate::application::file_dag::frontiers::types::FrontierInput;
use crate::application::file_dag::frontiers::types::FrontierSource;
use crate::application::file_dag::program::facade_runtime::service::ProjectFrontiers;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
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
///
/// Per rule of the phase, the files it has no frame for.
type Input = [&'static [FileId]; 2];
/// Per rule, the error opening the shared phase execution refused it with, if any.
type Value = [Option<ryo_execution_domain::domain::operating::errors::OperatingError>; 2];
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("project_rules_with_different_inapplicable_files_share_one_phase_execution"),
    word: Word::Succeeds,
    input: || [&[FileId(0)], &[]],
    expect: Expect::Value([None, None]),
}];

/// Opens the project phase for each rule with its required partitions.
fn subject(inapplicable: Input) -> Result<Value, Failure> {
    let PreparedCase {
        prepared,
        services,
        corpus,
    } = prepare_case(PhaseExit::Complete).map_err(|error| format!("{error:?}"))?;
    let phase = crate::application::file_dag::program::facade_runtime::service::publication_owner(
        &prepared,
        &services,
        Arc::new(crate::outbound::digest::fixtures::fake::DigestFake),
    )
    .map_err(|error| format!("{error:?}"))?;
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
        .map_err(|error| format!("{error:?}"))
    };
    let source = PlanInput::Relation(Relation::LanguageTarget);
    let source_root = prepared.program().rules()[0].frontier[0].clone();
    let partials = Partials::default();
    let mut symbols = Symbols::default();
    let mut opened = [None, None];
    for (rule, (open, files)) in opened.iter_mut().zip(inapplicable).enumerate() {
        let required = required(files)?;
        let table = Table {
            kinds: kinds(Relation::LanguageTarget).to_vec(),
            cells: vec![
                Cell::File(FileId(0)),
                Cell::Number(Integer(2)),
                Cell::Number(Integer(9)),
            ],
            count: RowCount(1),
        };
        *open = crate::application::file_dag::program::facade_runtime::project_execution::service::open_project(
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
        .err();
    }
    Ok(opened)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
