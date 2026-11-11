//! The support of the `sticky_refusal` cases.
use super::super::fixtures::REFUSAL;
use crate::outbound::operators::interface::OperatorsPort as _;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};

#[derive(Debug)]
pub(super) struct KeepsCalling {
    pub(super) refuse_project: bool,
}

impl crate::outbound::operators::interface::OperatorsPort for KeepsCalling {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        match call {
            OperatorCall::Evaluate {
                operation: ryo_values::domain::plans::types::Operation::Project { .. },
                ..
            } if self.refuse_project => Err(PortableOperatorFault::NoSuchColumn {
                index: ryo_values::domain::plan_parts::types::ColumnIndex(REFUSAL),
            }),
            OperatorCall::Evaluate {
                operation: ryo_values::domain::plans::types::Operation::Join { .. },
                driver,
                ..
            } => {
                let driver = driver.join();
                if self.refuse_project {
                    assert!(driver.emit(&[Cell::Number(Integer(1))]).is_err());
                    assert!(driver.emit(&[Cell::Number(Integer(2))]).is_err());
                    assert!(driver.next(&[Cell::Number(Integer(7))]).is_err());
                } else {
                    // External delivery begins only after this semantic result
                    // commits; it cannot interrupt this driver retroactively.
                    assert!(driver.emit(&[Cell::Number(Integer(1))]).is_ok());
                    assert!(driver.emit(&[Cell::Number(Integer(2))]).is_ok());
                }
                Ok(OperatorAnswer::Written)
            }
            call => crate::application::file_dag::fixtures::operators().operators(call),
        }
    }
}
