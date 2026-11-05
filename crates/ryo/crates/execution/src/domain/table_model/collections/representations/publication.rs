//! Generation descriptors contain no resident provider borrows or scheduling capability.
use crate::domain::stored_selection::types::Selector;
use std::sync::Arc;

/// Opaque immutable publication crossing the engine's domain port. A foreign
/// implementation carries no authority: the application verifies its private
/// record and current population epoch before admission or resolution.
pub(crate) trait PublishedRelation: std::any::Any + std::fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
}
/// Reuse the existing selector compiler; neither member owns execution state.
#[derive(Clone, Debug, Default)]
pub(crate) struct PreparedPhysical {
    pub selector: Option<Arc<Selector>>,
}
