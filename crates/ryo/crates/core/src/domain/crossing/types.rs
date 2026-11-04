//! The types of the `crossing` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::coverage::types::{Covered, Partial};
use crate::domain::table_model::types::ParameterBinding;
use crate::domain::trust::types::Needs;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::terms::types::UnknownName;
use ryo_values::domain::files::types::CorpusEntry;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stored_facts::types::ArenaBytes;

/// What the cross-file steps of every rule read besides the rows the files handed over.
#[derive(Clone, Debug)]
pub(crate) struct CrossingInput<'a> {
    /// Actual generation-bound readiness beside the producer frontier words.
    pub observations: &'a [crate::domain::coverage::types::FileObservationReadiness],
    /// The plans of the rules.
    pub plans: &'a [Plan],
    /// What each rule binds its parameters to.
    pub bindings: &'a [Vec<ParameterBinding>],
    /// The ids of the rules.
    pub ids: &'a [RuleId],
    /// The relations the steps of each rule trust the absence of rows of.
    pub needs: &'a [Needs],
    /// The path of each file of the run.
    pub paths: &'a [FilePath],
    /// The files, in the order of their paths.
    pub files: &'a [CorpusEntry],
    /// The files that have a relation read in part.
    pub covered: &'a [Covered],
    /// Files explicitly skipped because their syntax was rejected.
    pub skipped: &'a [Skipped],
    /// The relations the project wrote that are read in part.
    pub project: &'a [Partial],
    /// The relations the host declares, with their rows.
    pub hosted: &'a [HostRelation],
    /// How the rows the rules emit are answered.
    pub answering: &'a crate::domain::render::types::Answering,
    /// What the reasons of the languages linked are called.
    pub unknown: &'a [UnknownName],
}

/// Whether a finding can have a place in a file: a stretch, a node or a name among its columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Placeability {
    /// It can, so the file its subject names is read.
    Placeable,
    /// It cannot, so the file is not read.
    Unplaceable,
}

/// Pure cross-file evaluation before source placement or rendering.
#[derive(Default)]
pub(crate) struct Evaluated {
    /// Actual Emit and independent named roots for this phase.
    pub roots: PhaseRoots,
    /// Work performed by each step.
    pub fed: Vec<crate::domain::operator::types::RowsFed>,
}

/// Independently demanded named output from one actual execution phase.
#[derive(Debug)]
pub(crate) struct ObservedRoot {
    /// Resolved position in the producer's compiled named-root list.
    pub output: ryo_values::domain::observations::types::ObservationIndex,
    /// Present only for a complete, admitted output; unavailable never means empty.
    pub table: Option<crate::domain::table_model::types::Table>,
    /// Actual producer readiness independent of diagnostic Emit.
    pub readiness: ryo_values::domain::observations::types::ProducerReadiness,
}
/// The outputs of one cross-file execution, with no fabricated cross-phase Emit.
#[derive(Debug, Default)]
pub(crate) struct PhaseRoots {
    /// Emit refusal does not erase an independent successfully produced named root.
    pub emit_error: Option<crate::domain::operating::errors::OperatingError>,
    /// Present only if Emit belongs to this execution phase.
    pub emit: Option<crate::domain::table_model::types::Table>,
    /// Only roots explicitly retained by request demand linking.
    pub observations: Vec<ObservedRoot>,
}

/// A portable named output. Readiness and words agree: only Complete carries words.
#[derive(Clone, Debug)]
pub(crate) struct PackedObservedRoot {
    pub output: ryo_values::domain::observations::types::ObservationIndex,
    pub words: Option<ryo_values::domain::stored_facts::types::KeptWords>,
    pub readiness: ryo_values::domain::observations::types::ProducerReadiness,
    /// Every arena actually referenced by encoded places, including inherited re-exports.
    pub arenas: Vec<ryo_relations::domain::relations::types::WorkerNumber>,
}
/// One closure owns its reserved arena and returns only owned bytes/words.
#[derive(Clone, Debug)]
pub(crate) struct CrossPublicationTransfer {
    pub worker: ryo_relations::domain::relations::types::WorkerNumber,
    pub arena: ArenaBytes,
    pub roots: Vec<PackedObservedRoot>,
}

/// One actual named input supplied to a cross task. Failure stays explicit and cannot be empty rows.
#[derive(Clone, Debug)]
pub(crate) struct BoundInputWords {
    pub input: ryo_values::domain::observations::types::ObservationBindingId,
    pub kinds: Vec<ryo_relations::domain::relations::types::CellKind>,
    pub union: Result<
        std::sync::Arc<crate::domain::operator::contract::types::OperatorContract>,
        crate::domain::operating::errors::OperatingError,
    >,
    pub words: Result<
        Vec<std::sync::Arc<ryo_values::domain::stored_facts::types::KeptWords>>,
        crate::domain::operating::errors::OperatingError,
    >,
}

/// One required source that cannot provide rows; never represented by an empty Table.
#[derive(Clone, Debug)]
pub(crate) struct RefusedFrontier {
    pub input: ryo_values::domain::plan_parts::types::Input,
    pub error: crate::domain::operating::errors::OperatingError,
}
