//! Data of one execution's shared services, its publication scope and its file roots.
use crate::application::file_dag::program::admission::service::AdmittedInput;
use crate::application::file_dag::program::admission::service::AvailableInput;

/// The slot of a bound observation among the observation regions of a program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct ObservationKey(pub(super) usize);
/// Where an ordinary project frontier sits among those a run was given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct OrdinaryPlace(pub(super) usize);
/// Whether a run already holds the input a source names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Provision {
    Provided,
    Absent,
}
/// A project source admitted once for the run, with the source it answers.
pub(super) struct CachedInput {
    pub(super) source: ryo_execution_domain::domain::interpreted_program::types::InputSource,
    pub(super) input: AvailableInput,
}

pub(super) struct FileRoot {
    pub(super) readiness: Option<ryo_values::domain::records::types::ProducerReadiness>,
    pub(super) publication: Option<AdmittedInput>,
}
