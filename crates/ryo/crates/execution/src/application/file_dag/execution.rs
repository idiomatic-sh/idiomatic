//! The injected semantic adapter. Acquisition and invocation state are elsewhere.
use self::constants::BRIDGE_NAME;
use super::executor::{KernelCall, PrimitiveExecutor};
use crate::application::file_dag::executor::{KernelAnswer, KernelFault};
use ryo_execution_domain::domain::operator::contract::types::PrimitiveSemantics;
use crate::outbound::operators::interface::OperatorsPort;
mod constants;
pub(crate) mod lease;
pub(crate) mod output;

#[derive(Debug)]
pub(crate) struct CommonOperatorEngine {
    operators: std::sync::Arc<dyn OperatorsPort>,
}
impl CommonOperatorEngine {
    pub(crate) fn new(operators: std::sync::Arc<dyn OperatorsPort>) -> Self {
        Self { operators }
    }
}
impl PrimitiveExecutor for CommonOperatorEngine {
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
            KernelCall::Physical { request } => request
                .dispatch(self.operators.as_ref())
                .map(KernelAnswer::Physical),
        }
    }
}
pub(crate) struct PrimitiveSemanticsBridge<'a> {
    /// The file's profile, the observer a primitive reports its progress to.
    #[cfg(feature = "diagnostic-row-progress")]
    pub(super) progress: &'a ryo_execution_store::application::file_dag::route_counts::types::RouteProfile,
    pub(super) port: &'a dyn PrimitiveExecutor,
    pub(super) permit: crate::application::file_dag::program::runtime::service::SemanticPermit<'a>,
    pub(super) binding: ryo_execution_domain::domain::executable_program::physical::PhysicalBinding<'a>,
    pub(super) failure: &'a std::sync::OnceLock<ryo_execution_domain::domain::operating::errors::OperatingError>,
}
impl PrimitiveSemantics for PrimitiveSemanticsBridge<'_> {
    #[cfg(feature = "diagnostic-row-progress")]
    fn progress(&self) -> &dyn ryo_execution_domain::domain::operator::contract::views::ProgressObserver {
        self.progress
    }
    fn operators(
        &self,
        meaning: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
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

impl std::fmt::Debug for PrimitiveSemanticsBridge<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(BRIDGE_NAME)
            .finish_non_exhaustive()
    }
}
