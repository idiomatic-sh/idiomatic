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
            KernelCall::Registered { request } => {
                request.dispatch().map_err(KernelFault::Refused)?;
                Ok(KernelAnswer::Complete)
            }
            KernelCall::Walk { request } => request
                .dispatch()
                .map(KernelAnswer::Walk)
                .map_err(KernelFault::Refused),
            KernelCall::NativeJoin { request } => request
                .dispatch(self)
                .map(|()| KernelAnswer::Complete)
                .map_err(KernelFault::Refused),
            KernelCall::Physical { request } => {
                request.dispatch(self.operators).map(KernelAnswer::Physical)
            }
        }
    }
}
pub(super) struct PrimitiveSemanticsBridge<'a> {
    pub(super) port: &'a dyn PrimitiveExecutor,
    pub(super) permit: super::program::admission::SemanticPermit<'a>,
    pub(super) binding: crate::domain::executable_program::physical::PhysicalBinding,
    pub(super) failure: &'a std::sync::OnceLock<crate::domain::operating::errors::OperatingError>,
}
impl PrimitiveSemantics for PrimitiveSemanticsBridge<'_> {
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
        let request = match self.permit.physical(&self.binding, meaning) {
            Ok(request) => request,
            Err(error) => {
                let _first = self.failure.set(error);
                return Err(PortableOperatorFault::Interrupted);
            }
        };
        match self.port.execute(KernelCall::Physical { request }) {
            Ok(KernelAnswer::Physical(answer)) => Ok(answer),
            Ok(KernelAnswer::Complete | KernelAnswer::Walk(_)) => {
                Err(PortableOperatorFault::ShapeMismatch)
            }
            Err(KernelFault::Portable(error)) => Err(error),
            Err(KernelFault::Refused(error)) => {
                let _first = self.failure.set(error);
                Err(PortableOperatorFault::Interrupted)
            }
        }
    }
}
