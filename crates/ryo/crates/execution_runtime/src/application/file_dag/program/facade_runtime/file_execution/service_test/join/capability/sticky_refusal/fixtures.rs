//! The support of the `sticky_refusal` cases.
use super::super::fixtures::REFUSAL;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use std::sync::Mutex;

/// What the join driver answered one call the fake made of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Driven {
    /// The driver took the call.
    Taken,
    /// The driver refused the call.
    Refused,
}

/// Operators that keep calling the join driver after a refusal, recording what it answered each call.
#[derive(Debug, Default)]
pub(super) struct KeepsCalling {
    pub(super) refuse_project: bool,
    pub(super) driven: Mutex<Vec<Driven>>,
}

/// The driver's answer to one call, as the fake records it.
fn driven<E>(answer: Result<(), E>) -> Driven {
    answer.err().map_or(Driven::Taken, |_| Driven::Refused)
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
                let mut answers = vec![
                    driven(driver.emit(&[Cell::Number(Integer(1))])),
                    driven(driver.emit(&[Cell::Number(Integer(2))])),
                ];
                // External delivery begins only after this semantic result commits; it cannot
                // interrupt this driver retroactively. After a refusal the fake keeps asking.
                if self.refuse_project {
                    answers.push(driven(driver.next(&[Cell::Number(Integer(7))]).map(|_| ())));
                }
                self.driven
                    .lock()
                    .expect("the fake's record is not poisoned")
                    .extend(answers);
                Ok(OperatorAnswer::Written)
            }
            call => crate::application::file_dag::fixtures::operators().operators(call),
        }
    }
}
