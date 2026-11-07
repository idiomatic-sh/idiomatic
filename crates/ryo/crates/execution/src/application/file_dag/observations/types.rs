//! The values the cross-file publication leases hold.
use crate::application::file_dag::program::preparation::types::ObservationMetadata;
use ryo_relations::domain::relations::types::{FileId, WorkerNumber};
use ryo_values::domain::{
    observations::validated::ValidatedFrontiers, records::types::ObservationIndex,
    stored_facts::types::ArenaBytes,
};
use std::rc::Rc;

pub(in crate::application::file_dag::observations) enum CrossRoot {
    Pending,
    FileFrontier {
        publication: crate::application::file_dag::relations::packed::service::PackedPublication,
        token: crate::application::file_dag::program::restored::types::ValidatedFrontier,
        arenas: Vec<WorkerNumber>,
    },
    Named {
        publication: crate::application::file_dag::relations::packed::service::PackedPublication,
        admitted: crate::application::file_dag::program::observed_publication::types::ObservedPublication,
        arenas: Vec<WorkerNumber>,
    },
    Unavailable,
}
/// One expected file with the frame validated for it.
pub(in crate::application::file_dag::observations) struct FileFrontiers<'a> {
    pub file: FileId,
    pub frontiers: ValidatedFrontiers<'a>,
}
/// Every expected file has one validated frame; each logical state retains its file identity.
pub(crate) struct FrontierPopulation<'a> {
    pub(in crate::application::file_dag::observations) frames: Vec<FileFrontiers<'a>>,
}
/// A checked physical frontier; aliases share this exact owned payload and common lease.
pub(crate) struct PreparedFrontier {
    pub(in crate::application::file_dag::observations) publication: crate::application::file_dag::relations::packed::service::PackedPublication,
    pub(in crate::application::file_dag::observations) token: crate::application::file_dag::program::restored::types::ValidatedFrontier,
    pub(in crate::application::file_dag::observations) arenas: Vec<WorkerNumber>,
}
/// The number of arena slots the invocation inherited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::observations) struct ArenaCount(pub usize);
/// How many retained publications reference one arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::observations) struct ReferenceCount(pub usize);
/// How many consumers still need one producer output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::observations) struct ConsumerCount(pub usize);
/// The place of one retained frontier among those a program names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::observations) struct FrontierSlot(pub usize);
/// Whether a consumer rule has finished with its inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::observations) enum FinishState {
    Pending,
    Finished,
}
/// Whether a logical readiness agrees with the independently admitted physical coverage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::observations) enum FrontierCompatibility {
    Compatible,
    Incompatible,
}
/// Whether a rule is among those the current run selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuleSelection {
    Selected,
    Unselected,
}
/// One output of a rule with the admitted publication it delivers.
pub(crate) struct AdmittedOutput {
    pub output: ObservationIndex,
    pub publication: crate::application::file_dag::program::observed_publication::types::ObservedPublication,
}
/// One invocation extends the existing arena vector; stable slot indices never move during execution.
pub(crate) struct CrossPublications<'a> {
    pub(in crate::application::file_dag::observations) metadata: &'a ObservationMetadata,
    pub(in crate::application::file_dag::observations) owner: Rc<crate::application::file_dag::relations::RelationOwner>,
    pub(in crate::application::file_dag::observations) arenas: &'a mut Vec<ArenaBytes>,
    pub(in crate::application::file_dag::observations) inherited: ArenaCount,
    pub(in crate::application::file_dag::observations) references: Vec<ReferenceCount>,
    pub(in crate::application::file_dag::observations) roots: Vec<Vec<CrossRoot>>,
    pub(in crate::application::file_dag::observations) remaining: Vec<Vec<ConsumerCount>>,
    pub(in crate::application::file_dag::observations) finished: Vec<FinishState>,
}
