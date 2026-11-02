//! Application-owned shared execution lifetime.
use super::relations::{RelationOwner, RelationResult};
use crate::domain::file_program::types::NodeId;
use ryo_language::domain::relations::types::Relation;
use std::rc::Rc;
use std::sync::Arc;
/// The domain executor asks its application owner for an already scheduled DAG
/// output. Implementations own generation state; primitives never schedule it.
pub(crate) trait DagExecution {
    /// Establish required rule-local liveness before any operator demand.
    fn relation_owner(&self) -> Rc<RelationOwner>;
    fn begin_rule(&self, rule: usize);
    fn owned(&self, rule: usize, step: usize) -> Option<NodeId>;
    fn demand<'f>(
        &self,
        node: NodeId,
        read: &dyn Fn(
            Relation,
        ) -> Result<
            crate::domain::table_model::types::Rows<'f>,
            crate::domain::operating::errors::OperatingError,
        >,
        context: &mut crate::domain::operating::types::OperatorContext<'_>,
    ) -> Result<Arc<RelationResult>, crate::domain::operating::errors::OperatingError>;
}
