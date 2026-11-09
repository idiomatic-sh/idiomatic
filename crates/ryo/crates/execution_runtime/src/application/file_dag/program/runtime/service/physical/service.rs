//! Semantic permits and the physical request that carries its validated authority.
use super::super::{PermittedPhysical, SemanticPermit};
use crate::outbound::operators::interface::OperatorsPort as _;
use ryo_execution_domain::domain::executable_program::types::RouteKind;
use ryo_execution_domain::domain::operating::errors::OperatingError;

impl<'scope> SemanticPermit<'scope> {
    pub(crate) fn operation(&self) -> &ryo_values::domain::plans::types::Operation {
        &self.program.regions()[self.region.0].instructions()[self.instruction.0].operation()
    }
    pub(crate) fn validate_live(&self) -> Result<(), OperatingError> {
        if self.live.live().is_ok() {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    pub(crate) fn physical<'call, 'meaning>(
        &'call self,
        binding: &ryo_execution_domain::domain::executable_program::physical::PhysicalBinding<'_>,
        meaning: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'meaning>,
    ) -> Result<PermittedPhysical<'call, 'meaning, 'scope>, OperatingError> {
        self.validate_live()?;
        use ryo_execution_domain::domain::executable_program::physical::{self, PhysicalRoute};
        let route = match self.strategy {
            RouteKind::Generic => PhysicalRoute::Registered,
            RouteKind::OrderedExistence => PhysicalRoute::Ordered(
                ryo_execution_domain::domain::executable_program::service::ordered_existence(
                    &self.program.regions()[self.region.0].route(),
                )
                .ok_or_else(|| OperatingError::WrongOperation)?,
            ),
        };
        let authority = physical::authorize(self.operation(), route, binding, &meaning)?;
        Ok(PermittedPhysical {
            permit: self,
            meaning,
            authority,
        })
    }
}
impl PermittedPhysical<'_, '_, '_> {
    pub(crate) fn dispatch(
        self,
        port: &dyn crate::outbound::operators::interface::OperatorsPort,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
        crate::application::file_dag::executor::KernelFault,
    > {
        if self.permit.live.live().is_err() {
            return Err(
                crate::application::file_dag::executor::KernelFault::Refused(
                    OperatingError::NotRead,
                ),
            );
        }
        let _authority = self.authority;
        port.operators(self.meaning)
            .map_err(crate::application::file_dag::executor::KernelFault::Portable)
    }
}
