//! The types of the `loading` concept: plain data, public fields and derives, with no constructor and no inherent impl.
//! A load reads a workspace's definitions one wave at a time: what to read next, what was read, and what was wrong.

use crate::domain::definitions::types::LoadedWorkspace;

/// A definition to read next: its release, the directory of its package, and its file within the package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wanted {
    /// The release.
    pub release: crate::domain::packages::types::Release,
    /// Its package's directory.
    pub directory: crate::domain::packages::types::PackageRoot,
    /// Its definition file, relative to the package.
    pub file: DefinitionFile,
}

/// A definition file, or a template's content file, relative to its package.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefinitionFile(pub Box<str>);

/// A definition file a load read: its package's directory and its path within it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReadFile {
    /// The package's directory.
    pub directory: crate::domain::packages::types::PackageRoot,
    /// The file, relative to the package.
    pub file: DefinitionFile,
}

/// What loading found wrong: a name no package resolves, in a sentence.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LoadProblem(pub Box<str>);

/// A workspace loaded: its definitions, every definition file read, and what was wrong.
#[derive(Clone, Debug)]
pub struct WorkspaceLoad {
    /// The workspace.
    pub workspace: LoadedWorkspace,
    /// Every definition file read, each one the check's answer rests on.
    pub read: Vec<ReadFile>,
    /// What loading found wrong.
    pub problems: Vec<LoadProblem>,
}

/// A workspace being loaded: what is read so far, what is still to read, and every definition file read.
#[derive(Clone, Debug)]
pub struct Loading {
    /// The workspace so far.
    pub workspace: LoadedWorkspace,
    /// The releases still to read.
    pub pending: Vec<crate::domain::packages::types::Release>,
    /// The releases asked for, read or pending.
    pub asked: std::collections::BTreeSet<crate::domain::packages::types::Release>,
    /// Every definition file read, by package directory and path, each one the check's answer rests on.
    pub read: Vec<ReadFile>,
    /// What loading found: names no package resolves.
    pub problems: Vec<LoadProblem>,
}

/// A definition read and parsed, and where it was read from: what the load records of it.
#[derive(Clone, Debug)]
pub struct Parsed {
    /// The definition.
    pub definition: crate::domain::definitions::types::Definition,
    /// Where it was read from.
    pub place: crate::domain::paths::types::RepoPath,
}

/// What the read of a fixture's project's own files answers before any definition is read.
pub(crate) struct ProjectInputs {
    /// The repository the project is.
    pub repository: crate::domain::files::types::MachinePath,
    /// The directory of its `tastemaker.toml`.
    pub directory: crate::domain::paths::types::RepoPath,
    /// Its `tastemaker.toml`.
    pub file: crate::domain::paths::types::RepoPath,
    /// Every file of the project.
    pub files: Vec<crate::domain::paths::types::RepoPath>,
    /// Its local `tastemaker.toml` files.
    pub locals: Vec<crate::domain::definitions::types::LocalText>,
    /// The `workspace` section of its `tastemaker.toml`.
    pub section: crate::domain::definitions::types::WorkspaceSection,
    /// Its `tastemaker.toml`, read.
    pub workspace: crate::domain::definitions::types::WorkspaceFile,
    /// What is wrong with its variables.
    pub variables: Vec<crate::domain::variables::types::VariableProblem>,
}
