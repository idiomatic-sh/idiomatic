//! The injected semantic adapter. Acquisition and invocation state are elsewhere.
use super::executor::{KernelAnswer, KernelCall, KernelFault, PrimitiveExecutor};
use crate::domain::operator::contract::types::PrimitiveSemantics;
use crate::outbound::operators::interface::OperatorsPort;
pub(in crate::application::file_dag) mod lease;
pub(in crate::application::file_dag) mod output;

#[derive(Debug)]
pub(crate) struct CommonOperatorEngine<'a> {
    operators: &'a dyn OperatorsPort,
}
impl<'a> CommonOperatorEngine<'a> {
    pub(crate) fn new(operators: &'a dyn OperatorsPort) -> Self {
        Self { operators }
    }
}
impl PrimitiveExecutor for CommonOperatorEngine<'_> {
    fn execute(&self, call: KernelCall<'_, '_, '_>) -> Result<KernelAnswer, KernelFault> {
        match call {
            KernelCall::Registered {
                permit,
                operation,
                inputs,
                lease,
                output,
            } => {
                crate::domain::executable_program::kernel::evaluate(
                    permit.kind(),
                    operation,
                    inputs,
                    lease,
                    output,
                )
                .map_err(KernelFault::Refused)?;
                Ok(KernelAnswer::Complete)
            }
            KernelCall::Physical { permit: _, meaning } => self
                .operators
                .operators(meaning)
                .map(KernelAnswer::Physical)
                .map_err(KernelFault::Portable),
        }
    }
}
pub(super) struct PrimitiveSemanticsBridge<'a, 'invocation> {
    pub(super) port: &'a dyn PrimitiveExecutor,
    pub(super) permit: &'a super::program::admission::KernelPermit<'invocation>,
    pub(super) failure:
        &'a std::cell::RefCell<Option<crate::domain::operating::errors::OperatingError>>,
}
impl PrimitiveSemantics for PrimitiveSemanticsBridge<'_, '_> {
    #[cfg(feature = "diagnostic-row-progress")]
    fn progress(&self) -> crate::domain::operator::contract::views::ProgressObserver {
        super::route_counts::progress
    }
    fn operators(
        &self,
        meaning: ryo_operators::domain::contract::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::domain::contract::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        use ryo_operators::domain::contract::types::PortableOperatorFault;
        match self.port.execute(KernelCall::Physical {
            permit: self.permit,
            meaning,
        }) {
            Ok(KernelAnswer::Physical(answer)) => Ok(answer),
            Ok(KernelAnswer::Complete) => Err(PortableOperatorFault::ShapeMismatch),
            Err(KernelFault::Portable(error)) => Err(error),
            Err(KernelFault::Refused(error)) => {
                self.failure.borrow_mut().get_or_insert(error);
                Err(PortableOperatorFault::Interrupted)
            }
        }
    }
}
