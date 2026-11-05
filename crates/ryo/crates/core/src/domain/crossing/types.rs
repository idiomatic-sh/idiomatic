//! The types of the `crossing` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::coverage::types::{Covered, Partial};
use crate::domain::trust::types::Needs;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::terms::types::UnknownName;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stored_facts::types::ArenaBytes;

/// What the cross-file steps of every rule read besides the rows the files handed over.
#[derive(Clone, Debug)]
pub(crate) struct CrossingInput<'a> {
    /// The same compiled file scopes used during per-file evaluation.
    pub placed: &'a crate::domain::placement::types::PlacedScopes,
    /// Actual generation-bound readiness beside the producer frontier words.
    pub observations: &'a [crate::domain::coverage::types::FileObservationReadiness],
    /// Pure source and accounting facts extracted before executable plans are consumed.
    pub metadata: &'a [crate::domain::run_metadata::types::RuleMetadata],
    /// The ids of the rules.
    pub ids: &'a [RuleId],
    /// The relations the steps of each rule trust the absence of rows of.
    pub needs: &'a [Needs],
    /// The path of each file of the run.
    pub paths: &'a [FilePath],
    /// The files, in the order of their paths.
    pub files: &'a [CorpusInput],
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
    pub table: Option<ryo_values::domain::rows::types::Table>,
    /// Actual producer readiness independent of diagnostic Emit.
    pub readiness: ryo_values::domain::observations::types::ProducerReadiness,
}
/// The outputs of one cross-file execution, with no fabricated cross-phase Emit.
#[derive(Debug, Default)]
pub(crate) struct PhaseRoots {
    /// Emit refusal does not erase an independent successfully produced named root.
    pub emit_error: Option<crate::domain::operating::errors::OperatingError>,
    /// Exact affected physical file ordinals for a usable partial Emit.
    /// An unlocated entry requires request-level refusal if no file can represent it.
    pub emit_affected: Option<Vec<Option<ryo_relations::domain::relations::types::FileId>>>,
    /// Present only if Emit belongs to this execution phase.
    pub emit: Option<ryo_values::domain::rows::types::Table>,
    /// Only roots explicitly retained by request demand linking.
    pub observations: Vec<ObservedRoot>,
}

/// One required source that cannot provide rows; never represented by an empty Table.
#[derive(Clone, Debug)]
pub(crate) struct RefusedFrontier {
    pub input: ryo_values::domain::plan_parts::types::Input,
    pub error: crate::domain::operating::errors::OperatingError,
}
