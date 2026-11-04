//! Application-owned shared execution lifetime.
use super::executor::ScopedExecutor;
use super::relations::{RelationOwner, RelationResult};
use crate::domain::file_program::types::NodeId;
use ryo_relations::domain::relations::types::Relation;
use std::rc::Rc;
use std::sync::Arc;
/// The domain executor asks its application owner for an already scheduled DAG
/// output. Implementations own generation state; primitives never schedule it.
pub(crate) trait DagExecution {
    /// Establish required rule-local liveness before any operator demand.
    fn relation_owner(&self) -> Rc<RelationOwner>;
    fn begin_rule(
        &self,
        rule: usize,
    ) -> Result<(), crate::domain::operating::errors::OperatingError>;
    fn owned(&self, rule: usize, step: usize) -> Option<NodeId>;
    /// Preserve the independent root's original refusal without erasing a completed Emit.
    fn refuse_observation(
        &self,
        rule: usize,
        output: ryo_values::domain::observations::types::ObservationIndex,
        reason: ryo_values::domain::observations::types::ObservationReason,
    ) -> Result<(), crate::domain::operating::errors::OperatingError>;
    /// Read already-published named roots; never execute the producer in a consumer context.
    fn observation<'f>(
        &self,
        slot: crate::domain::file_program::types::ObservationSlot,
        read: &dyn Fn(
            Relation,
        ) -> Result<
            crate::domain::table_model::types::Rows<'f>,
            crate::domain::operating::errors::OperatingError,
        >,
        executor: &ScopedExecutor<'_>,
        context: &mut crate::domain::operating::types::OperatorContext<'_>,
    ) -> Result<
        crate::application::file_dag::relations::PublishedInput,
        crate::domain::operating::errors::OperatingError,
    >;
    fn demand<'f>(
        &self,
        node: NodeId,
        read: &dyn Fn(
            Relation,
        ) -> Result<
            crate::domain::table_model::types::Rows<'f>,
            crate::domain::operating::errors::OperatingError,
        >,
        executor: &ScopedExecutor<'_>,
        context: &mut crate::domain::operating::types::OperatorContext<'_>,
    ) -> Result<
        crate::application::file_dag::relations::PublishedInput,
        crate::domain::operating::errors::OperatingError,
    >;
}
