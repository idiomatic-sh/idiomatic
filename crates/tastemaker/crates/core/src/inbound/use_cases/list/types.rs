//! The types of the `list` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::entries::types::Description;
use crate::domain::files::types::MachinePath;
use crate::domain::packages::types::{DefinitionKind, DefinitionName, PackageName, VersionText};

/// Which kinds of definition are listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kinds {
    /// Every kind.
    All,
    /// One kind.
    Only(DefinitionKind),
    /// The relations of ryo's catalog a query may read.
    Relations,
    /// The operators of ryo's catalog a query may call.
    Operators,
}

/// What `list` is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListRequest {
    /// Where to look for the workspace.
    pub start: MachinePath,
    /// Which kinds.
    pub kinds: Kinds,
    /// Only the definitions whose name or description holds this text; empty means every one.
    pub term: SearchTerm,
}

/// The text a listed definition's name or description holds; empty means every one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchTerm(pub Box<str>);

/// What a listed item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ListedKind {
    /// A definition.
    Definition(DefinitionKind),
    /// A relation of ryo's catalog: no package and no version.
    Relation,
    /// An operator of ryo's catalog: no package and no version.
    Operator,
}

/// One definition listed, at its latest version.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Listed {
    /// What it is.
    pub kind: ListedKind,
    /// Its package.
    pub package: PackageName,
    /// Its name.
    pub name: DefinitionName,
    /// Its latest version.
    pub version: VersionText,
    /// What it is, on one line.
    pub description: Description,
}

/// What `list` answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListReport {
    /// The definitions, sorted.
    pub listed: Vec<Listed>,
}
