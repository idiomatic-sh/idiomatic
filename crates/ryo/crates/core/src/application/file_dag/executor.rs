//! Mandatory engine boundary. The domain delivery algorithm never chooses an operator policy.
use crate::application::file_dag::feed::Feed;
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, OperatorContext, Room},
};
use crate::domain::operator::contract::types::OperatorContract;
use crate::domain::table_model::types::Rows;
use std::{rc::Rc, sync::Arc};

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
        owner: &Rc<RelationOwner>,
        contract: &OperatorContract,
        feed: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError>;
    fn collect(
        &self,
        owner: &Rc<RelationOwner>,
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
        owner: &Rc<RelationOwner>,
        context: &mut OperatorContext<'_>,
    ) -> Result<crate::application::file_dag::relations::PublishedInput, OperatingError> {
        let mut table = Table::default();
        crate::domain::table_model::service::reset(
            &mut table,
            contract.collected_kinds.as_ref().map_err(Clone::clone)?,
        );
        self.stream(
            owner,
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
        let owner = Rc::clone(&stepper.relation_owner);
        self.stream(&owner, contract, stepper, context, room, emit)
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
        let owner = Rc::clone(&stepper.relation_owner);
        self.stream(
            &owner,
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

/// A file-generation (or explicitly standalone) invocation boundary over request-wide metadata.
/// Neither dependency is optional, and cloning this value shares the exact same owner.
#[derive(Clone)]
pub(crate) struct ScopedExecutor<'a> {
    executor: &'a dyn PrimitiveExecutor,
    owner: Rc<RelationOwner>,
}
impl std::fmt::Debug for ScopedExecutor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScopedExecutor")
            .field("executor", &self.executor)
            .finish_non_exhaustive()
    }
}
impl<'a> ScopedExecutor<'a> {
    pub(crate) fn new(
        executor: &'a dyn PrimitiveExecutor,
        owner: Rc<RelationOwner>,
    ) -> Result<Self, OperatingError> {
        if !Arc::ptr_eq(&executor.derived_port(), &owner.derived.semantic_port()) {
            return Err(OperatingError::WrongOperation);
        }
        Ok(Self { executor, owner })
    }
    pub(crate) fn owner(&self) -> &Rc<RelationOwner> {
        &self.owner
    }
    pub(crate) fn derived_port(&self) -> Arc<dyn crate::outbound::derived::interface::DerivedPort> {
        self.owner.derived.semantic_port()
    }
    pub(crate) fn stream(
        &self,
        contract: &OperatorContract,
        feed: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        if !Rc::ptr_eq(&self.owner, &feed.relation_owner()) {
            return Err(OperatingError::NotRead);
        }
        self.executor
            .stream(&self.owner, contract, feed, context, room, emit)
    }
    pub(crate) fn collect(
        &self,
        contract: &OperatorContract,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        self.executor
            .collect(&self.owner, contract, inputs, context, out)
    }
    pub(crate) fn keep_representation(
        &self,
        contract: &OperatorContract,
        physical: &crate::domain::table_model::collections::representations::publication::PreparedPhysical,
        feed: &mut GraphInputs<'_, '_>,
        context: &mut OperatorContext<'_>,
    ) -> Result<crate::application::file_dag::relations::PublishedInput, OperatingError> {
        if !Rc::ptr_eq(&self.owner, &feed.relation_owner()) {
            return Err(OperatingError::NotRead);
        }
        self.executor
            .keep_representation(contract, physical, feed, &self.owner, context)
    }
    pub(crate) fn stream_step<'b, 'f>(
        &self,
        contract: &OperatorContract,
        stepper: &mut crate::application::file_dag::pipeline::types::Stepper<'b, 'f>,
        id: crate::domain::operator::types::StepPlace,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError>
    where
        'f: 'b,
    {
        if !Rc::ptr_eq(&self.owner, &stepper.relation_owner) {
            return Err(OperatingError::NotRead);
        }
        self.executor
            .stream_step(contract, stepper, id, context, room, emit)
    }
    pub(crate) fn keep_step<'b, 'f>(
        &self,
        contract: &OperatorContract,
        stepper: &mut crate::application::file_dag::pipeline::types::Stepper<'b, 'f>,
        id: crate::domain::operator::types::StepPlace,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        target: &mut Table,
    ) -> Result<StepMaterialization, OperatingError>
    where
        'f: 'b,
    {
        if !Rc::ptr_eq(&self.owner, &stepper.relation_owner) {
            return Err(OperatingError::NotRead);
        }
        self.executor
            .keep_step(contract, stepper, id, context, room, target)
    }
}
