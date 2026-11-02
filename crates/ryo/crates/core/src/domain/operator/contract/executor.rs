//! Mandatory engine boundary. The domain delivery algorithm never chooses an operator policy.
use super::types::OperatorContract;
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, Feed, OperatorContext, Room},
};
use crate::domain::table_model::types::{Rows, Table};
pub(crate) enum StepMaterialization {
    Materialized,
    Selected(std::sync::Arc<crate::domain::table_model::types::StoredSelection>),
}
pub(crate) trait PrimitiveExecutor: std::fmt::Debug + Sync {
    fn stream(
        &self,
        contract: &OperatorContract,
        feed: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError>;
    fn collect(
        &self,
        contract: &OperatorContract,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError>;

    fn stream_step<'a, 'f>(
        &self,
        contract: &OperatorContract,
        stepper: &mut crate::domain::pipeline::types::Stepper<'a, 'f>,
        id: crate::domain::operator::types::StepPlace,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError>
    where
        'f: 'a,
    {
        self.stream(contract, stepper, context, room, emit)
    }
    fn keep_step<'a, 'f>(
        &self,
        contract: &OperatorContract,
        stepper: &mut crate::domain::pipeline::types::Stepper<'a, 'f>,
        _id: crate::domain::operator::types::StepPlace,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        target: &mut Table,
    ) -> Result<StepMaterialization, OperatingError>
    where
        'f: 'a,
    {
        self.stream(
            contract,
            stepper,
            context,
            room,
            &mut crate::domain::replay::types::Collect(target),
        )?;
        Ok(StepMaterialization::Materialized)
    }
}
