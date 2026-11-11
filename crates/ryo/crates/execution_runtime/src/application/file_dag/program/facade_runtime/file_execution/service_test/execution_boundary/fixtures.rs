//! The one-file request the execution-boundary proofs run through the real owner.
use super::super::super::service::*;
use super::super::fixtures::*;
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{Input as PlanInput, StepId},
    plans::types::Operation,
    rows::types::Table,
    stored_facts::types::Digest,
};
use std::sync::Arc;
pub(crate) fn source(rows: Rows<'_>) -> Result<Table, OperatingError> {
    let mut plan = plan(PlanShape::ProjectRetry);
    plan.steps.truncate(1);
    plan.emit.step = StepId(0);
    plan.steps[0].operation = Operation::Select {
        from: PlanInput::Relation(Relation::SyntaxTree),
        comparisons: vec![],
        kept: vec![],
    };
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
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("execution-boundary.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let mut symbols = Symbols::default();
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
        &mut symbols,
        None,
        None,
    )?;
    let read = |relation| {
        if relation == Relation::SyntaxTree {
            Ok(rows.clone())
        } else {
            Err(OperatingError::NotRead)
        }
    };
    let outcome = file.rule(
        RuleIndex(0),
        RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::NotRequired,
        },
        &read,
        &mut symbols,
    )?;
    let result = match outcome.emit {
        RootOutcome::Complete(table) => Ok(table),
        RootOutcome::Failed(error) | RootOutcome::Unknown(error) => Err(error),
        _ => Err(OperatingError::NotRun),
    };
    file.finish()?;
    result
}
