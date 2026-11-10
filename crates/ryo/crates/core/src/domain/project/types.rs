//! The types of the `project` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_execution::inbound::use_cases::metadata::interface::{Partial, Unknown};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::runs::types::ManifestRefused;
use ryo_values::domain::stores::types::StoredFile;

/// Whether the cross-file steps of a plan read a thing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlanReads {
    /// They read it.
    Reads,
    /// They do not.
    DoesNotRead,
}

/// What the project came to: the manifests the tools refused and the relations it wrote that are
/// read in part.
#[derive(Clone, Debug, Default)]
pub(crate) struct Projected {
    /// Exact entry observations consumed by project readers in this run.
    pub observed_entries: Vec<ryo_language::domain::project_context::types::EntryObservation>,
    /// The relations of the project read in part.
    pub partials: Vec<Partial>,
    /// The manifests the languages' tools refused.
    pub refused: Vec<ManifestRefused>,
    /// The manifests the rows are about that the run does not list, numbered after the files it lists in this order: nothing when the
    /// project was not made in this run.
    pub unlisted: Vec<FilePath>,
    /// What the rules that read the project could not decide for its partly read relations: nothing until the use case says.
    pub unknowns: Vec<Unknown>,
    /// The files the lookup read beyond the manifests the run handed it, in the order of their paths, each with its stamps and
    /// digest as the store keeps them: those it read now when the project was made in this run, those the store kept otherwise.
    pub consulted: Vec<StoredFile>,
    /// The files the lookup asked for and found none of, in the order of their paths.
    pub consulted_absent: Vec<FilePath>,
}
