//! Semantic permits and the physical request that carries its validated authority.
use super::super::{KernelStrategy, PermittedPhysical, SemanticPermit};
use crate::domain::operating::errors::OperatingError;

impl<'scope> SemanticPermit<'scope> {
    pub(in crate::application::file_dag::program::runtime::service) fn operation(
        &self,
    ) -> &ryo_values::domain::plans::types::Operation {
        &self.program.regions()[self.region.0].instructions[self.instruction].operation
    }
    pub(in crate::application::file_dag::program::runtime::service) fn validate_live(
        &self,
    ) -> Result<(), OperatingError> {
        if self.live.is_live() {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    pub(in crate::application::file_dag) fn physical<'call, 'meaning>(
        &'call self,
        binding: &crate::domain::executable_program::physical::PhysicalBinding<'_>,
        meaning: ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'meaning>,
    ) -> Result<PermittedPhysical<'call, 'meaning, 'scope>, OperatingError> {
        self.validate_live()?;
        use crate::domain::executable_program::physical::{self, PhysicalRoute};
        let route = match self.strategy {
            KernelStrategy::Registered => PhysicalRoute::Registered,
            KernelStrategy::Positions => PhysicalRoute::Positions(
                self.program.regions()[self.region.0]
                    .native_positions
                    .as_ref()
                    .ok_or(OperatingError::WrongOperation)?,
            ),
            KernelStrategy::Ordered => PhysicalRoute::Ordered(
                self.program.regions()[self.region.0]
                    .ordered_existence
                    .as_ref()
                    .ok_or(OperatingError::WrongOperation)?,
            ),
            KernelStrategy::Column => PhysicalRoute::Column(
                self.program.regions()[self.region.0]
                    .column
                    .as_ref()
                    .ok_or(OperatingError::WrongOperation)?,
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
    pub(in crate::application::file_dag) fn dispatch(
        self,
        port: &dyn crate::outbound::operators::interface::OperatorsPort,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::interface::OperatorAnswer,
        crate::application::file_dag::executor::response::types::KernelFault,
    > {
        if !self.permit.live.is_live() {
            return Err(
                crate::application::file_dag::executor::response::types::KernelFault::Refused(
                    OperatingError::NotRead,
                ),
            );
        }
        let _authority = self.authority;
        port.operators(self.meaning)
            .map_err(crate::application::file_dag::executor::response::types::KernelFault::Portable)
    }
}
