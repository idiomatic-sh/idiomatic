//! The support of the `refusal` cases.

#[derive(Debug)]
pub(super) struct RefusingSemantics;

impl crate::outbound::operators::interface::OperatorsPort for RefusingSemantics {
    fn operators(
        &self,
        _: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        Err(ryo_operators::domain::contract::types::PortableOperatorFault::WrongOperation)
    }
}
