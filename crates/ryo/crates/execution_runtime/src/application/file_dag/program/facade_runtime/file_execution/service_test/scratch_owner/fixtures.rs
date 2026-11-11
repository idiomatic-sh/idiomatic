//! The support of the `scratch_owner` cases.
use super::super::super::*;
use super::super::fixtures::*;
use crate::application::file_dag::execution::CommonOperatorEngine;
use ryo_execution_domain::domain::operating::fixtures::ExecutionObservation;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::StepId,
    plans::types::Operation,
    rows::types::Table,
    stored_facts::types::Digest,
};
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::HostedNumber,
    row_texts::types::Cell as HostedCell,
};
use std::{rc::Rc, sync::Arc};

pub(crate) fn hosted(kinds: &[HostKind], rows: Vec<Vec<HostedCell>>) -> HostRelation {
    HostRelation {
        name: HostName("scratch-input".into()),
        columns: kinds
            .iter()
            .enumerate()
            .map(|(at, kind)| HostColumn {
                name: HostColumnName(format!("value-{at}").into()),
                kind: *kind,
            })
            .collect(),
        rows: rows.into_iter().map(HostRow).collect(),
    }
}

pub(crate) fn execute(
    operation: Operation,
    host: HostRelation,
    symbols: &mut Symbols,
) -> (Result<Table, OperatingError>, ExecutionObservation) {
    let mut plan = plan(Scenario::ProjectRetry);
    plan.steps[0].operation = Operation::Hosted {
        relation: HostedNumber(0),
    };
    plan.steps[1].operation = operation;
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![plan],
            bindings: vec![vec![]],
        },
        &[rule()],
        &[Digest([0; 16])],
        &[host],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )
    .unwrap();
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(std::sync::Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("scratch-owner.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }],
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ReuseEligibility::Eligible,
        symbols,
        None,
        None,
    )
    .unwrap();
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    let outcome = file
        .rule(
            RuleIndex(0),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &read,
            symbols,
        )
        .unwrap();
    let observed = file.route_observation(RuleIndex(0), StepId(1)).unwrap();
    let context = file
        .context
        .borrow(symbols, &prepared, RuleIndex(0))
        .unwrap();
    assert!(
        context.scratch.select_scratch.0.get().is_none(),
        "dynamic matcher state belongs to the actual invocation, not reusable rule scratch"
    );
    let owner = Rc::clone(&file.owner);
    file.finish().unwrap();
    assert_eq!(
        owner.derived().working_bytes(),
        0,
        "normal file shutdown releases actual required resources"
    );
    let result = match outcome.emit {
        RootOutcome::Complete(table) => Ok(table),
        RootOutcome::Unknown(error) | RootOutcome::Failed(error) => Err(error),
        _ => Err(OperatingError::NotRun),
    };
    (result, observed)
}
