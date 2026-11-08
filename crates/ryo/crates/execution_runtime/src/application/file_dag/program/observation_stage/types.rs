//! One project publication generation shared by every ready batch.
use crate::application::file_dag::program::project_phase::types::ProjectPhase;
/// One project publication generation shared by every ready batch.
pub struct ObservationStage<'a> {
    pub(crate) phase: ProjectPhase<'a>,
    pub(crate) publications:
        crate::application::file_dag::observations::types::CrossPublications<'a>,
}
