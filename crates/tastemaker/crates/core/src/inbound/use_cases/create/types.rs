//! The types of the `create` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::files::types::MachinePath;
use crate::domain::packages::types::{DefinitionKind, DefinitionName};
use crate::domain::paths::types::RepoPath;
use crate::domain::writing::types::Writing;

/// Which package a definition is created in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    /// The workspace's first package by path.
    Workspace,
    /// The package at this directory.
    Package(MachinePath),
}

/// What `create` is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateRequest {
    /// Where to look for the workspace.
    pub start: MachinePath,
    /// The kind.
    pub kind: DefinitionKind,
    /// The name.
    pub name: DefinitionName,
    /// The package.
    pub into: Destination,
    /// The extension the skeleton's scenario files end in, as the person names it: a rule and a template need one, and
    /// Tastemaker never guesses it.
    pub extension: Extension,
    /// Whether it writes.
    pub writing: Writing,
}

/// Whether the person named the extension a skeleton's scenario files end in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Extension {
    /// This one.
    Named(ScenarioExtension),
    /// None: a rule or a template is then refused, never given a guessed one.
    Unnamed,
}

/// The extension a new rule's or template's scenario files end in, as the person names it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScenarioExtension(pub Box<str>);

/// What `create` answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateReport {
    /// The package's directory.
    pub package: MachinePath,
    /// The file written, or that would be, relative to the package.
    pub file: RepoPath,
    /// The kind created.
    pub kind: DefinitionKind,
    /// Its name.
    pub name: DefinitionName,
}
