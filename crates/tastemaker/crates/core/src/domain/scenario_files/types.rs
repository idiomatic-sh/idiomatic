//! The types of the `scenario_files` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeMap;

use crate::domain::entries::types::{Count, ParamName, Value};
use crate::domain::packages::types::{DefinitionKind, DefinitionName};
use crate::domain::selectors::types::{PathPattern, Selectors};

/// Where an entry places a template: no file, for a rule or a spec, or the file it names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileKey {
    /// A rule's or a spec's entry, which places by `include`.
    Unnamed,
    /// A template's entry, placing it at this file.
    Named(PathPattern),
}

/// The count an entry writes: none where it is what the file's path says, or this one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CountKey {
    /// What the path says: a literal path is one, a glob or capture at least one.
    Derived,
    /// This count.
    Written(Count),
}

/// The keys of the entry that places what a scenario proves, beyond the definition it names: a list or a table only
/// when it holds something.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioKeys {
    /// Where a rule or a spec is placed.
    pub include: Selectors,
    /// What is taken from `include`.
    pub exclude: Selectors,
    /// Where a template is placed.
    pub file: FileKey,
    /// How many files a template places.
    pub count: CountKey,
    /// What the entry passes.
    pub params: BTreeMap<ParamName, Value>,
}

/// The root `tastemaker.toml` of a scenario without its own: the whole project, open, depending on the package of the
/// definition at `path`, with one entry naming the definition, extended by the scenario's keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScenarioRoot<'a> {
    /// Where the package is.
    pub path: &'a crate::domain::packages::types::PackageRoot,
    /// The definition the entry names.
    pub placed: &'a crate::domain::packages::types::PlacedDefinition,
    /// The scenario's keys.
    pub keys: &'a ScenarioKeys,
}

/// The wrapper spec an owned definition is proved through: one entry naming the definition by its name, extended by the
/// scenario's keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wrapper<'a> {
    /// The owned definition's kind: rule or template.
    pub kind: DefinitionKind,
    /// Its name.
    pub name: &'a DefinitionName,
    /// The spec that owns it.
    pub spec: &'a DefinitionName,
    /// The scenario's keys.
    pub keys: &'a ScenarioKeys,
}

/// The text of a `tastemaker.toml` a scenario writes among its files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkspaceText<'a>(pub &'a str);

/// A `tastemaker.toml` a scenario writes among its files: where it is written in the scenario's project, and its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WrittenWorkspace<'a> {
    /// Its path in the scenario's project.
    pub path: &'a crate::domain::paths::types::RepoPath,
    /// Its text.
    pub text: WorkspaceText<'a>,
}

/// What places the definition a scenario proves, when the scenario's files bring no `tastemaker.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScenarioEntry {
    /// An entry with these keys, written for the scenario.
    Keys(ScenarioKeys),
    /// None is written: the scenario's own `tastemaker.toml` places it, or the project is its own whole.
    Brought,
}

/// Whether a fixture's project brings its own `tastemaker.toml`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnWorkspace {
    /// It does, and it was written.
    Written,
    /// It does not: one is written for it.
    Absent,
}
