//! The types of the `closing` concept: plain data, public fields and derives, with no constructor and no inherent impl. The facts
//! placement leaves of the closed directories, and the strays the closed query decides from them.

use std::collections::BTreeSet;

use crate::domain::findings::types::FindingRule;
use crate::domain::paths::types::RepoPath;
use crate::domain::scopes::types::{Identity, ScopeIndex};

/// The number a closed directory has among the closed directories of a placement, from zero, in the order of their paths.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupNumber(pub u32);

/// The number a file beneath a closed directory has among those of every closed directory, from zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrayNumber(pub u32);

/// How many directories a directory is below the root.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DirectoryDepth(pub u32);

/// A directory as a prefix of the paths beneath it: the path and a `/`, none for the root.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Prefix(pub Box<str>);

/// A scope that is closed at a directory, and how findings name it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosedOwner {
    /// The scope.
    pub scope: ScopeIndex,
    /// How findings name it.
    pub identity: Identity,
}

/// A directory that is closed, and the scopes that close it, in the order they are declared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosedDirectory {
    /// Its number.
    pub group: GroupNumber,
    /// Its path.
    pub directory: RepoPath,
    /// The scopes that close it.
    pub owners: Vec<ClosedOwner>,
}

/// A file beneath a closed directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnclosedFile {
    /// Its number.
    pub stray: StrayNumber,
    /// The closed directory it is beneath.
    pub group: GroupNumber,
    /// Its path.
    pub path: RepoPath,
    /// The directory that holds it.
    pub home: Prefix,
}

/// An instance nested in a closed directory: it owns its directory, which the closed one does not account for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nested {
    /// The closed directory.
    pub group: GroupNumber,
    /// The instance's directory.
    pub prefix: Prefix,
}

/// A directory one scope placed files in, once for each scope and directory.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OccupiedDirectory {
    /// The scope.
    pub owner: ScopeIndex,
    /// The directory, a path from the root of the workspace.
    pub directory: RepoPath,
}

/// A directory a scope that closes a directory placed files in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Occupancy {
    /// The closed directory.
    pub group: GroupNumber,
    /// The scope.
    pub owner: ScopeIndex,
    /// The directory it placed files in.
    pub prefix: Prefix,
    /// How many directories that is below the root.
    pub depth: DirectoryDepth,
}

/// What placement leaves of the closed directories: the facts the closed query decides the strays from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Closings {
    /// Every check that ran.
    pub evaluated: BTreeSet<FindingRule>,
    /// The closed directories.
    pub directories: Vec<ClosedDirectory>,
    /// The files beneath them.
    pub candidates: Vec<EnclosedFile>,
    /// The files templates placed.
    pub placed: Vec<RepoPath>,
    /// The `tastemaker.toml` files.
    pub own: Vec<RepoPath>,
    /// The instances nested in them.
    pub nested: Vec<Nested>,
    /// The directories their scopes placed files in.
    pub occupied: Vec<Occupancy>,
}

/// The scope a stray is named after, as the closed query decided it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Stray {
    /// The file.
    pub stray: StrayNumber,
    /// The scope.
    pub owner: ScopeIndex,
}
