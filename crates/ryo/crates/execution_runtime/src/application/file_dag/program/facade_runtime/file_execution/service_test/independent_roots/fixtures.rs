//! The support of the `independent_roots` cases.
use crate::outbound::operators::interface::OperatorsPort as _;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::observations::types::{ObservationColumn, ObservationColumnName};
use ryo_values::domain::plans::types::Operation;

#[derive(Debug)]
pub(super) struct RefuseCount;

impl crate::outbound::operators::interface::OperatorsPort for RefuseCount {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        if matches!(
            &call,
            OperatorCall::ReduceCount { .. }
                | OperatorCall::Evaluate {
                    operation: Operation::GroupCount { .. },
                    ..
                }
        ) {
            return Err(PortableOperatorFault::WrongOperation);
        }
        crate::application::file_dag::fixtures::operators().operators(call)
    }
}

pub(super) fn columns(failed_emit: bool) -> Vec<ObservationColumn> {
    let mut columns = vec![ObservationColumn {
        name: ObservationColumnName("subject".into()),
        kind: CellKind::Node,
    }];
    if !failed_emit {
        columns.push(ObservationColumn {
            name: ObservationColumnName("count".into()),
            kind: CellKind::Number,
        });
    }
    columns
}
