//! Owned resources for one declared project occurrence, before engine admission.
use ryo_execution::application::file_dag::frontiers::types::BoundInput;
use ryo_execution::application::file_dag::frontiers::types::PreparedFrontiers;
use ryo_execution::application::file_dag::program::project_phase::types::PhaseScope;
use ryo_execution::application::file_dag::frontiers::types::PublicationSlot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OccurrenceNumber(pub usize);

pub(crate) struct PreparedProject {
    pub scope: PhaseScope,
    pub frontier: PreparedFrontiers,
    pub bound: Vec<BoundInput>,
    pub slot: PublicationSlot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProjectOccurrenceCount(pub usize);
