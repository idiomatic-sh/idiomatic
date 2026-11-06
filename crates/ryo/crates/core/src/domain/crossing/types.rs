//! The types of the `crossing` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_execution::inbound::use_cases::metadata::interface::Needs;
use ryo_execution::inbound::use_cases::metadata::interface::{Covered, Partial};
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::terms::types::UnknownName;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stored_facts::types::ArenaBytes;

/// What the cross-file steps of every rule read besides the rows the files handed over.
#[derive(Clone, Debug)]
pub(crate) struct CrossingInput<'a> {
    /// The same compiled file scopes used during per-file evaluation.
    pub placed: &'a ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes,
    /// Actual generation-bound readiness beside the producer frontier words.
    pub observations:
        &'a [ryo_execution::inbound::use_cases::metadata::interface::FileObservationReadiness],
    /// Pure source and accounting facts extracted before executable plans are consumed.
    pub metadata: &'a [ryo_values::domain::run_metadata::types::RuleRunMetadata],
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
    /// The files no language read, and why.
    pub unread: &'a [ryo_values::domain::records::types::Unread],
    /// The relations the project wrote that are read in part.
    pub project: &'a [Partial],
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

/// Independently demanded named output from one actual execution phase.
#[derive(Debug)]
pub(crate) struct ObservedRoot {
    /// Resolved position in the producer's compiled named-root list.
    pub output: ryo_values::domain::records::types::ObservationIndex,
    /// Present only for an admitted complete or partial output; unavailable never means empty.
    pub table: Option<ryo_values::domain::rows::types::Table>,
    /// Actual producer readiness independent of diagnostic Emit.
    pub readiness: ryo_values::domain::records::types::ProducerReadiness,
}
/// The outputs of one cross-file execution, with no fabricated cross-phase Emit.
#[derive(Debug, Default)]
pub(crate) struct PhaseRoots {
    /// Emit refusal does not erase an independent successfully produced named root.
    pub emit_error: Option<ryo_execution::inbound::use_cases::execution::types::OperatingError>,
    /// Exact affected physical file ordinals for a usable partial Emit.
    /// An unlocated entry requires request-level refusal if no file can represent it.
    pub emit_affected: Option<Vec<Option<ryo_relations::domain::relations::types::FileId>>>,
    /// Present only if Emit belongs to this execution phase.
    pub emit: Option<ryo_values::domain::rows::types::Table>,
    /// Only roots explicitly retained by request demand linking.
    pub observations: Vec<ObservedRoot>,
}
