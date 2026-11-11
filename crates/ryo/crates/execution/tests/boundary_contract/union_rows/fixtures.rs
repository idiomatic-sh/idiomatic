//! The support of the `union_rows` cases and of the groups nested in it.
use super::super::*;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::HostedNumber,
    row_texts::types::Cell as HostCell,
};

pub(super) fn program(
    engine: &PrepareUseCaseImpl,
    left: &[i64],
    right: &[i64],
) -> PreparedExecutionProgram {
    let mut input = authored_request(1);
    crate::boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Hosted {
                relation: HostedNumber(0),
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Hosted {
                relation: HostedNumber(1),
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Union {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
            },
        },
    ];
    crate::boundary_fixture::fixtures::plans(&mut input)[0]
        .emit
        .step = StepId(2);
    input.hosted = [left, right]
        .into_iter()
        .enumerate()
        .map(|(side, rows)| HostRelation {
            name: HostName(format!("union-side-{side}").into()),
            columns: vec![HostColumn {
                name: HostColumnName("value".into()),
                kind: HostKind::Number,
            }],
            rows: rows
                .iter()
                .map(|value| HostRow(vec![HostCell::Number(Integer(*value))]))
                .collect(),
        })
        .collect();
    engine.prepare(input).expect("actual Union program")
}
