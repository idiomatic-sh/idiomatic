//! The support of the `rule_outputs` cases.
use super::super::super::fixtures::*;
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::hosted::types::HostKind;
use ryo_values::domain::plan_parts::types::HostedNumber;
use ryo_values::domain::row_texts::types::Cell as HostedCell;
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows, StepId},
    plans::types::Operation,
    runs::types::RuleId,
    stored_facts::types::Digest,
};
use std::sync::Arc;

pub(super) fn output_requests(
    outputs: &[(&str, usize)],
    bad_first: bool,
) -> (
    Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    Arc<ProgramServices>,
) {
    use ryo_values::domain::hosted::types::{
        HostColumn, HostColumnName, HostName, HostRelation, HostRow,
    };
    let hosts = outputs
        .iter()
        .map(|(name, count)| HostRelation {
            name: HostName(format!("output-{name}").into()),
            columns: vec![HostColumn {
                name: HostColumnName("value".into()),
                kind: HostKind::Number,
            }],
            rows: (0..*count)
                .map(|_| HostRow(vec![HostedCell::Number(Integer(7))]))
                .collect(),
        })
        .collect::<Vec<_>>();
    let plans = outputs
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let mut plan = plan(Scenario::ProjectRetry);
            plan.steps[0].operation = Operation::Hosted {
                relation: HostedNumber(index as u32),
            };
            plan.steps[1].operation = Operation::Project {
                from: PlanInput::Step(StepId(0)),
                columns: vec![ColumnIndex(if bad_first && index == 0 { 99 } else { 0 })],
                distinct: RepeatedRows::Kept,
            };
            plan
        })
        .collect();
    let rules = outputs
        .iter()
        .map(|(name, _)| {
            let mut rule = rule();
            rule.id = RuleId(format!("output-{name}").into());
            rule
        })
        .collect::<Vec<_>>();
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]; outputs.len()],
            plans,
            bindings: vec![vec![]; outputs.len()],
        },
        &rules,
        &vec![Digest([0; 16]); outputs.len()],
        &hosts,
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )
    .unwrap();
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("owned-outputs.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    (prepared, services)
}
