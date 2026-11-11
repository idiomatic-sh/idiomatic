//! The types of the `execution` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use super::super::executor::{service::PrimitiveExecutor, types::KernelCall};
use super::constants::BRIDGE_NAME;
use crate::application::file_dag::executor::{types::KernelAnswer, types::KernelFault};
use crate::outbound::operators::interface::OperatorsPort;
use ryo_execution_domain::domain::operating::types::PrimitiveSemantics;

pub(crate) struct PrimitiveSemanticsBridge<'a> {
    /// The file's profile, the observer a primitive reports its progress to.
    #[cfg(feature = "diagnostic-row-progress")]
    pub(crate) progress:
        &'a ryo_execution_store::application::file_dag::route_counts::profile::service::RouteProfile,
    pub(crate) port: &'a dyn PrimitiveExecutor,
    pub(crate) permit: crate::application::file_dag::program::runtime::service::SemanticPermit<'a>,
    pub(crate) binding:
        ryo_execution_domain::domain::executable_program::physical::PhysicalBinding<'a>,
    pub(crate) failure:
        &'a std::sync::OnceLock<ryo_execution_domain::domain::operating::errors::OperatingError>,
}

impl PrimitiveSemantics for PrimitiveSemanticsBridge<'_> {
    #[cfg(feature = "diagnostic-row-progress")]
    fn progress(&self) -> &dyn ryo_execution_domain::domain::operating::types::ProgressObserver {
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
        formatter.debug_struct(BRIDGE_NAME).finish_non_exhaustive()
    }
}
