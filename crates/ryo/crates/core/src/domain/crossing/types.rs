//! The types of the `crossing` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::coverage::types::{Covered, Partial};
use crate::domain::table_model::types::ParameterBinding;
use crate::domain::trust::types::Needs;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::terms::types::UnknownName;
use ryo_values::domain::files::types::CorpusEntry;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::ArenaBytes;

/// What the cross-file steps of every rule read besides the rows the files handed over.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CrossingInput<'a> {
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
    /// The text each worker made.
    pub arenas: &'a [ArenaBytes],
    /// The files, in the order of their paths.
    pub files: &'a [CorpusEntry],
    /// The files that have a relation read in part.
    pub covered: &'a [Covered],
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
    /// Evaluated rows.
    pub table: crate::domain::table_model::types::Table,
    /// Work performed by each step.
    pub fed: Vec<crate::domain::operator::types::RowsFed>,
}
