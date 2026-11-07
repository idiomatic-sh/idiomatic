//! The types of the `completion` part: what a project phase delivers and what it publishes.
use crate::domain::answering::types::Crossed;
use crate::domain::crossing::types::PhaseRoots;
use ryo_execution::application::file_dag::program::observed_publication::types::ObservedPublication;
use ryo_execution::application::file_dag::frontiers::types::PublicationSlot;
use ryo_execution::inbound::use_cases::execution::interface::RowsFed;
use ryo_values::domain::records::types::ObservationIndex;

/// The publication of one named output, with the output it belongs to.
pub(crate) struct OutputPublication {
    /// The named output the publication was admitted for.
    pub output: ObservationIndex,
    /// The actual admitted owner proof of the output's usable root.
    pub publication: ObservedPublication,
}

/// Pure phase rows plus the actual admitted owner proof of each usable named root.
pub(crate) struct ProgramEvaluated {
    /// The diagnostic and named roots the phase delivered.
    pub roots: PhaseRoots,
    /// The rows each step fed.
    pub fed: Vec<RowsFed>,
    /// The publication of each usable named root.
    pub publications: Vec<OutputPublication>,
}

/// Diagnostic result plus the actual admitted identities of its complete and partial named roots.
pub(crate) struct CrossedWithPublications {
    /// The publication slot the phase reserved.
    pub slot: PublicationSlot,
    /// What the phase answered.
    pub crossed: Crossed,
    /// The publication of each named root that was published.
    pub publications: Vec<OutputPublication>,
}
