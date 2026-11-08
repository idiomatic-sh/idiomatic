//! Population identity data shared by admitted source and derived owners.
/// Assigned only after application admission; canonical DAG producers keep one ID across rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PopulationId(
    pub(in crate::application::file_dag) usize,
);
