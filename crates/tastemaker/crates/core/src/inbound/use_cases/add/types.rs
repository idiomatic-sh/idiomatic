//! The types of the `add` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeMap;

use crate::domain::entries::types::Setting;
use crate::domain::files::types::MachinePath;
use crate::domain::packages::types::{DefinitionKind, DefinitionName};
use crate::domain::paths::types::RepoPath;
use crate::domain::writing::types::Writing;

/// What `add` is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddRequest {
    /// Where to look for the workspace.
    pub start: MachinePath,
    /// The definition: `name`, or `package/name`.
    pub definition: DefinitionAsked,
    /// Where it applies, relative to the start: a rule's or spec's directory, a template's file.
    pub at: RepoPath,
    /// The values it passes, by name.
    pub params: BTreeMap<Setting, Setting>,
    /// Whether it writes.
    pub writing: Writing,
}

/// What `add` answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddReport {
    /// The `tastemaker.toml` written, or that would be.
    pub file: RepoPath,
    /// The entry's definition, as `package/name@version`.
    pub entry: ShownEntry,
    /// The entry's name, which `scaffold` takes next.
    pub reference: DefinitionName,
    /// What it names: a rule places no file, so nothing is scaffolded for it.
    pub kind: DefinitionKind,
    /// Whether the entry is new to the file, or the file already held it and the add leaves it as it was.
    pub novelty: Novelty,
}

/// Whether an added entry is new to the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Novelty {
    /// The file did not hold the entry.
    New,
    /// The file already held the entry.
    Held,
}

/// A definition as the caller names it: `name`, or `package/name`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionAsked(pub Box<str>);

/// An entry's definition as it is shown: `package/name@version`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShownEntry(pub Box<str>);
