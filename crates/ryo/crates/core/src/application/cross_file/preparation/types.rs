//! Owned resources for one declared project occurrence, before engine admission.
use ryo_execution::inbound::use_cases::execution::types::{
    BoundInput, PreparedFrontiers, ProjectScope, PublicationSlot,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OccurrenceNumber(pub usize);

pub(crate) struct PreparedProject {
    pub scope: ProjectScope,
    pub frontier: PreparedFrontiers,
    pub bound: Vec<BoundInput>,
    pub slot: PublicationSlot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProjectOccurrenceCount(pub usize);
