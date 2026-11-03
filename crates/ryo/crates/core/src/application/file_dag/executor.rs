//! Mandatory engine boundary. The domain delivery algorithm never chooses an operator policy.
use crate::application::file_dag::feed::Feed;
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, OperatorContext, Room},
};
use crate::domain::operator::contract::types::OperatorContract;
use crate::domain::table_model::types::Rows;
use std::sync::Arc;

use super::{
    relations::{RelationOwner, RelationResult},
    service::GraphInputs,
};
pub(crate) enum StepMaterialization {
    Materialized,
    Selected(std::sync::Arc<crate::domain::table_model::types::StoredSelection>),
}
pub(crate) trait PrimitiveExecutor: std::fmt::Debug + Sync {
    fn derived_port(&self) -> Arc<dyn crate::outbound::derived::interface::DerivedPort>;

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

    /// Keep one DAG result through the same injected primitive executor used
    /// by ordinary pipeline steps. A custom executor retains its own refusal
    /// and observation semantics; the common executor may publish a richer
    /// representation through the application owner.
    fn keep_representation(
        &self,
        contract: &OperatorContract,
        _physical: &crate::domain::table_model::collections::representations::publication::PreparedPhysical,
        feed: &mut GraphInputs<'_, '_>,
        owner: &RelationOwner,
        context: &mut OperatorContext<'_>,
    ) -> Result<crate::application::file_dag::relations::PublishedInput, OperatingError> {
        let mut table = Table::default();
        crate::domain::table_model::service::reset(
            &mut table,
            contract.collected_kinds.as_ref().map_err(Clone::clone)?,
        );
        self.stream(
            contract,
            feed,
            context,
            &mut Room::default(),
            &mut crate::domain::replay::types::Collect(&mut table),
        )?;
        owner.computed(table)
    }

    fn stream_step<'a, 'f>(
        &self,
        contract: &OperatorContract,
        stepper: &mut crate::application::file_dag::pipeline::types::Stepper<'a, 'f>,
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
        stepper: &mut crate::application::file_dag::pipeline::types::Stepper<'a, 'f>,
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

use ryo_values::domain::rows::types::Table;
