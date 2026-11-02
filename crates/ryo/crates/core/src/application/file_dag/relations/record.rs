//! Engine-private publication and admission state. Domain representations cannot mint it.
use super::{
    OperatingError, PhysicalShape, PublishedRelation, RelationBody, RepresentationKind, Table,
};
use std::sync::{Arc, OnceLock};
/// Only this application module can construct or mutate proof state.
#[derive(Debug)]
pub(crate) struct RelationResult {
    pub(super) population: usize,
    pub(super) producer: OnceLock<usize>,
    pub(super) generation: Arc<()>,
    pub(super) kind: RepresentationKind,
    pub(super) body: RelationBody,
    pub(super) inspected: OnceLock<Result<PhysicalShape, OperatingError>>,
    pub(super) normalized: OnceLock<Arc<Table>>,
}
impl PublishedRelation for RelationResult {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn kind(&self) -> RepresentationKind {
        self.kind
    }
}
