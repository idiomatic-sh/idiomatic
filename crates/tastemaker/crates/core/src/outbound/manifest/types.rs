//! The types of the `manifest` port: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::entries::types::{Count, Description, ParamValues};
use crate::domain::packages::types::{DefinitionKind, DefinitionName, PackageName, VersionText};
use crate::domain::selectors::types::{PathPattern, Selectors};

/// An entry to write into a `tastemaker.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryDraft {
    /// What it names.
    pub kind: DefinitionKind,
    /// Its package.
    pub package: PackageName,
    /// Its version.
    pub version: VersionText,
    /// Its definition.
    pub name: DefinitionName,
    /// Where: a rule's or spec's `include`.
    pub include: Selectors,
    /// A template's `file`; empty for a rule or a spec.
    pub file: PathPattern,
    /// A spec's or template's count.
    pub count: Count,
    /// What it passes.
    pub params: ParamValues,
    /// What it is here.
    pub description: Description,
}

/// The text of a definition file, as read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionText(pub Box<str>);
