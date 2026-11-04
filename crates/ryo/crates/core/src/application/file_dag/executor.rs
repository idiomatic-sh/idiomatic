//! Mandatory engine boundary. The domain delivery algorithm never chooses an operator policy.
use crate::application::file_dag::feed::Feed;
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, OperatorContext, Room},
};
use crate::domain::operator::contract::types::OperatorContract;
use crate::domain::table_model::types::Rows;
use std::{rc::Rc, sync::Arc};

use super::{relations::RelationOwner, service::GraphInputs};
/// A common-owner declaration of the source and canonical outputs of one
/// shared native physical batch. It carries no dependency scheduler or cache.
pub(crate) struct NativeClassBatch<'a> {
    pub graph: &'a crate::domain::file_program::types::RequestGraph,
    pub input: ryo_relations::domain::relations::types::Relation,
    pub outputs: Vec<crate::domain::file_program::types::NodeId>,
}
pub(crate) trait PrimitiveExecutor: std::fmt::Debug + Sync {
    /// Required invocation boundary: custom executors must explicitly preserve
    /// their refusal/observation policy for the planned physical alternative.
    fn ordered_existence(
        &self,
        owner: &Rc<RelationOwner>,
        shape: &crate::domain::ordered_existence::types::ResolvedShape,
        source: &super::relations::PublishedInput,
        rows: crate::domain::operator::contract::views::BorrowedRows<'_>,
        output_kinds: &[ryo_relations::domain::relations::types::CellKind],
        context: &mut OperatorContext<'_>,
        room: &mut Room,
    ) -> Result<super::ordered_existence::StrategyOutcome, OperatingError>;

    /// Required even for custom executors: native precomputation cannot bypass
    /// refusal/observation by running before the ordinary node entry.
    fn native_classes(
        &self,
        owner: &Rc<RelationOwner>,
        declaration: &NativeClassBatch<'_>,
        shared: &mut crate::domain::shared_walk::types::MergedWalk,
        walking: crate::domain::shared_walk::types::Walking<'_>,
    ) -> Result<(), OperatingError>;

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
    /// Required graph-owned streaming/physical invocation; no Stepper can enter
    /// this interface. Custom executors explicitly retain refusal/observation.
    fn stream_representation(
        &self,
        contract: &OperatorContract,
        physical: &crate::domain::table_model::collections::representations::publication::PreparedPhysical,
        feed: &mut GraphInputs<'_, '_>,
        owner: &Rc<RelationOwner>,
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
        physical: &crate::domain::table_model::collections::representations::publication::PreparedPhysical,
        feed: &mut GraphInputs<'_, '_>,
        owner: &Rc<RelationOwner>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
    ) -> Result<crate::application::file_dag::relations::PublishedInput, OperatingError>;
}

/// An explicit ordinary materialization choice, not an overridable default.
/// Custom executors must name this choice or implement the declared physical region.
pub(crate) fn keep_rows<E: PrimitiveExecutor + ?Sized>(
    executor: &E,
    contract: &OperatorContract,
    feed: &mut GraphInputs<'_, '_>,
    owner: &Rc<RelationOwner>,
    context: &mut OperatorContext<'_>,
    room: &mut Room,
) -> Result<super::relations::PublishedInput, OperatingError> {
    if feed.project_phase() {
        return feed.collect_with(owner, context, |inputs, context, out| {
            executor.collect(owner, contract, inputs, context, out)
        });
    }
    let mut table = Table::default();
    crate::domain::table_model::service::reset(
        &mut table,
        contract.collected_kinds.as_ref().map_err(Clone::clone)?,
    );
    executor.stream(
        owner,
        contract,
        feed,
        context,
        room,
        &mut crate::domain::replay::types::Collect(&mut table),
    )?;
    owner.computed(table)
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
    pub(super) fn ordered_existence(
        &self,
        shape: &crate::domain::ordered_existence::types::ResolvedShape,
        source: &super::relations::PublishedInput,
        rows: crate::domain::operator::contract::views::BorrowedRows<'_>,
        output_kinds: &[ryo_relations::domain::relations::types::CellKind],
        context: &mut OperatorContext<'_>,
        room: &mut Room,
    ) -> Result<super::ordered_existence::StrategyOutcome, OperatingError> {
        self.executor.ordered_existence(
            &self.owner,
            shape,
            source,
            rows,
            output_kinds,
            context,
            room,
        )
    }
    pub(super) fn native_classes(
        &self,
        declaration: &NativeClassBatch<'_>,
        shared: &mut crate::domain::shared_walk::types::MergedWalk,
        walking: crate::domain::shared_walk::types::Walking<'_>,
    ) -> Result<(), OperatingError> {
        self.executor
            .native_classes(&self.owner, declaration, shared, walking)
    }
    pub(crate) fn owner(&self) -> &Rc<RelationOwner> {
        &self.owner
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
    pub(crate) fn stream_representation(
        &self,
        contract: &OperatorContract,
        physical: &crate::domain::table_model::collections::representations::publication::PreparedPhysical,
        feed: &mut GraphInputs<'_, '_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        if !Rc::ptr_eq(&self.owner, &feed.relation_owner()) {
            return Err(OperatingError::NotRead);
        }
        self.executor.stream_representation(
            contract,
            physical,
            feed,
            &self.owner,
            context,
            room,
            emit,
        )
    }

    #[cfg(test)]
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
        room: &mut Room,
    ) -> Result<crate::application::file_dag::relations::PublishedInput, OperatingError> {
        if !Rc::ptr_eq(&self.owner, &feed.relation_owner()) {
            return Err(OperatingError::NotRead);
        }
        self.executor
            .keep_representation(contract, physical, feed, &self.owner, context, room)
    }
}
