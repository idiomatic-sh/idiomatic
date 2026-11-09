//! The admitted population an observation hands over.
/// Opaque application handover of the actual admitted population: what was read and how completely, its producer root, the
/// publication and the population and generation it was admitted under. Packed bytes may accompany it, but never substitute for
/// this evidence.
#[derive(Clone)]
pub struct ObservedPublication {
    pub(crate) proof: crate::application::file_dag::program::admission::ReadProof,
    pub(crate) root: crate::application::file_dag::program::admission::ProducerRoot,
    pub(crate) publication:
        ryo_execution_store::application::file_dag::relations::handle::service::Publication,
    pub(crate) population:
        ryo_execution_store::application::file_dag::derived::population::types::PopulationId,
    pub(crate) generation: ryo_execution_store::application::file_dag::relations::generation::service::ResidentGeneration,
}
