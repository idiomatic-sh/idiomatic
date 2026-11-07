//! One project publication generation shared by every ready batch.
use crate::application::file_dag::program::project_phase::types::ProjectPhase;
/// One project publication generation shared by every ready batch.
pub struct ObservationStage<'a> {
    pub(in crate::application::file_dag::program) phase: ProjectPhase<'a>,
    pub(in crate::application::file_dag::program) publications:
        crate::application::file_dag::observations::types::CrossPublications<'a>,
}
