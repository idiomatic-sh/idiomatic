//! The types of the `publications` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::application::file_dag::program::observed_publication::service::ObservedPublication;
use crate::application::file_dag::program::preparation::types::ObservationMetadata;
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_operators::domain::contract::types::WithheldInput;
use ryo_relations::domain::relations::types::{CellKind, FileId, WorkerNumber};
use ryo_values::domain::observations::types::validated_frontiers::value::ValidatedFrontiers;
use ryo_values::domain::plan_parts::types::ObservationBindingId;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::stored_facts::types::{ArenaBytes, KeptWords};
use std::sync::Arc;

/// What one named output of a rule has published in the stage.
#[derive(Debug)]
pub(crate) enum CrossRoot {
    /// It runs after the files and has not published yet.
    Pending,
    /// It runs in each file: what the files kept of it, restored for its consumers.
    Restored(RestoredOutput),
    /// It ran after the files and its occurrence installed it.
    Named {
        /// Its rows and their coverage.
        publication: ObservedPublication,
        /// The arenas its texts are in.
        arenas: Vec<WorkerNumber>,
    },
    /// It has nothing to publish.
    Unavailable,
}

/// A named output each file kept, proven against the files its rule read: its words, decoded at its consumers' first binding.
#[derive(Debug)]
pub(crate) struct RestoredOutput {
    /// The words the files kept.
    pub(crate) words: Arc<KeptWords>,
    /// What each column holds.
    pub(crate) kinds: Box<[CellKind]>,
    /// How completely the files covered it.
    pub(crate) coverage: CoverageState,
    /// The arenas its texts are in.
    pub(crate) arenas: Vec<WorkerNumber>,
    /// Its rows in the stage's symbols, once a consumer bound it.
    pub(crate) decoded: std::cell::OnceCell<Result<Arc<Table>, OperatingError>>,
}

/// One expected file with the frame validated for it.
pub(crate) struct FileFrontiers<'a> {
    /// The file.
    pub(crate) file: FileId,
    /// Its frame.
    pub(crate) frontiers: ValidatedFrontiers<'a>,
}

/// Every expected file has one validated frame; each logical state retains its file identity.
pub(crate) struct FrontierPopulation<'a> {
    /// Each file's frame, in file order.
    pub(crate) frames: Vec<FileFrontiers<'a>>,
}

/// The number of arena slots the invocation inherited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ArenaCount(pub(crate) usize);

/// How many retained publications reference one arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReferenceCount(pub(crate) usize);

/// How many consumers still need one producer output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ConsumerCount(pub(crate) usize);

/// The place of one retained frontier among those a program names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrontierSlot(pub(crate) usize);

/// Whether a consumer rule has finished with its inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FinishState {
    /// Not yet.
    Pending,
    /// It has.
    Finished,
}

/// Whether a logical readiness agrees with the independently admitted physical coverage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FrontierCompatibility {
    /// It agrees.
    Compatible,
    /// It does not.
    Incompatible,
}

/// Whether a rule is among those the current run selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuleSelection {
    /// Selected.
    Selected,
    /// Not selected.
    Unselected,
}

/// Whether one occurrence reserved a publication arena.
#[derive(Clone, Copy, Debug)]
pub(crate) enum PublicationSlot {
    /// It publishes nothing after the files.
    Unused,
    /// Its arena, reserved in the stage's arenas.
    Reserved(WorkerNumber),
}

/// The tables one occurrence's bound observations are bound to, by binding.
#[derive(Debug, Default)]
pub(crate) struct BoundTables(pub(crate) Vec<BoundTable>);

/// One bound observation's rows: its members' rows in member order, each row once, or why it is withheld.
#[derive(Debug)]
pub(crate) struct BoundTable {
    /// The binding.
    pub(crate) input: ObservationBindingId,
    /// Its rows.
    pub(crate) rows: BoundRows,
}

/// A bound observation's rows and their coverage, or the token of why they are withheld.
#[derive(Debug)]
pub(crate) enum BoundRows {
    /// The rows and their coverage.
    Held {
        /// The rows.
        table: Table,
        /// Their coverage.
        coverage: CoverageState,
    },
    /// Withheld: the phase answers the token with the reason it recorded.
    Withheld(WithheldInput),
}

/// The stage's publications: each occurrence's slot and install, the arenas, the restored outputs and how many consumers each
/// output has left. One invocation extends the arena vector it inherits; slot indices never move during it.
pub(crate) struct Publications<'a> {
    /// The program's named outputs, bindings and retirements.
    pub(crate) metadata: &'a ObservationMetadata,
    /// The arenas, the inherited ones first.
    pub(crate) arenas: &'a mut Vec<ArenaBytes>,
    /// How many arenas were inherited.
    pub(crate) inherited: ArenaCount,
    /// How many retained publications reference each arena.
    pub(crate) references: Vec<ReferenceCount>,
    /// What each output of each rule published.
    pub(crate) roots: Vec<Vec<CrossRoot>>,
    /// How many consumers each output of each rule has left.
    pub(crate) remaining: Vec<Vec<ConsumerCount>>,
    /// Whether each rule has finished with its inputs.
    pub(crate) finished: Vec<FinishState>,
    /// The stage's life: each publication the stage makes holds it weakly, so one held past the stage is refused.
    pub(crate) stage: Arc<()>,
}
