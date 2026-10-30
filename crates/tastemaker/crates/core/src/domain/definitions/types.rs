//! The types of the `definitions` concept: plain data, public fields and derives, with no constructor and no inherent impl.
//! The four files as Tastemaker reads them: `tastemaker.toml`, `spec.toml`, `rule.toml` and
//! `template.toml`.

use std::collections::BTreeMap;

use crate::domain::entries::types::{
    Composition, Described, Description, ParamDeclarations, ParamName, Value,
};
use crate::domain::findings::types::WorkspaceName;
use crate::domain::packages::types::{Dependency, PackageName};
use crate::domain::scenarios::types::{RuleScenario, SpecScenario, TemplateScenario};
use crate::domain::selectors::types::Selectors;

/// Why something exists: what someone needs before using it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Why(pub Box<str>);

/// The `workspace` table of the root `tastemaker.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceSection {
    /// Its name.
    pub name: WorkspaceName,
    /// The paths no rule reads; they are still placed.
    pub rules_exclude: Selectors,
    /// The directories that may hold a local `tastemaker.toml`.
    pub members: Selectors,
    /// The packages used directly.
    pub dependencies: Vec<Dependency>,
}

/// Whether a `tastemaker.toml` is the workspace's root file or a local one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Role {
    /// The root file, with its `workspace` table.
    Root(WorkspaceSection),
    /// A local file under a member directory.
    Local,
}

/// A `tastemaker.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceFile {
    /// Root or local.
    pub role: Role,
    /// What it composes.
    pub composition: Composition,
}

/// A `spec.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecFile {
    /// What kind of directory it describes.
    pub description: Description,
    /// What it is for.
    pub why: Why,
    /// The params it accepts.
    pub params: ParamDeclarations,
    /// The paths of an instance its rules do not read.
    pub rules_exclude: Selectors,
    /// What it composes.
    pub composition: Composition,
    /// Its scenarios.
    pub scenarios: Vec<SpecScenario>,
}

/// RyoQL text.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueryText(pub Box<str>);

/// A column of a rule's query.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColumnName(pub Box<str>);

use crate::domain::findings::types::MessageTemplate;

/// A rule's diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// The column a finding is placed at.
    pub primary: ColumnName,
    /// What is wrong.
    pub message: MessageTemplate,
    /// How to fix it.
    pub help: MessageTemplate,
}

/// The values one exception admits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exception {
    /// The values the query receives as `$exceptions`.
    pub admits: Vec<Value>,
    /// Why they are admitted.
    pub why: Why,
}

/// A `rule.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleFile {
    /// What it checks.
    pub description: Description,
    /// Why it matters.
    pub why: Why,
    /// Its params.
    pub params: ParamDeclarations,
    /// Its named exceptions.
    pub exceptions: BTreeMap<ParamName, Exception>,
    /// Its query.
    pub query: QueryText,
    /// Its diagnostic.
    pub diagnostic: Diagnostic,
    /// Its scenarios.
    pub scenarios: Vec<RuleScenario>,
}

/// Whether a template's content is the whole file, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Exactness {
    /// The file may hold more than the content: the default, `match = "contains"`.
    Contains,
    /// The file holds exactly the content's declarations, in order: `match = "declarations"`.
    Exact,
    /// The file's text is the content's, byte for byte, as well as its declarations: `match = "text"`.
    Verbatim,
}

/// A template's content: minijinja text.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemplateText(pub Box<str>);

/// A `template.toml`, its content read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateFile {
    /// What the file is.
    pub description: Description,
    /// What it is for.
    pub why: Why,
    /// The params its content uses.
    pub params: ParamDeclarations,
    /// Whether the content is the whole file.
    pub exact: Exactness,
    /// Its content.
    pub content: TemplateText,
    /// Its scenarios.
    pub scenarios: Vec<TemplateScenario>,
}

/// One definition, of any kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Definition {
    /// A rule.
    Rule(RuleFile),
    /// A spec.
    Spec(SpecFile),
    /// A template.
    Template(TemplateFile),
}

/// The kind of a v2 `layers`, `independent` or `acyclic` key a file writes, for the finding that says to convert it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FoldedKey {
    /// `layers`.
    Layers,
    /// `independent`.
    Independent,
    /// `acyclic`.
    Acyclic,
}

/// The package a workspace builds from a local path inside its own tree, whose files `validate` governs.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalPackage {
    /// The package.
    pub package: PackageName,
    /// Its directory in the repository.
    pub directory: crate::domain::paths::types::RepoPath,
}

/// Whether a definition explains itself: a description and a why that are not blank.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Explained {
    /// Both are written.
    Explained,
    /// This one is blank.
    Blank(Described),
}

/// A local `tastemaker.toml`: one under a directory the root's `members` names. Its paths start at its own directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalFile {
    /// The file.
    pub file: crate::domain::paths::types::RepoPath,
    /// Its directory.
    pub directory: crate::domain::paths::types::RepoPath,
    /// What it composes.
    pub composition: Composition,
}

/// A workspace read: its files, its packages, every release its entries name, and the files it governs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedWorkspace {
    /// The root `tastemaker.toml`.
    pub file: crate::domain::paths::types::RepoPath,
    /// Its directory: the project root.
    pub directory: crate::domain::paths::types::RepoPath,
    /// Its `workspace` table.
    pub section: WorkspaceSection,
    /// What it composes.
    pub composition: Composition,
    /// The local files.
    pub locals: Vec<LocalFile>,
    /// The packages resolved, direct and transitive.
    pub packages: BTreeMap<PackageName, crate::domain::packages::types::ResolvedPackage>,
    /// Every spec release the entries name.
    pub specs: BTreeMap<crate::domain::packages::types::Release, SpecFile>,
    /// Every rule release.
    pub rules: BTreeMap<crate::domain::packages::types::Release, RuleFile>,
    /// Every template release, its content read.
    pub templates: BTreeMap<crate::domain::packages::types::Release, TemplateFile>,
    /// The highest version each shared release's package offers.
    pub latest: BTreeMap<
        crate::domain::packages::types::Release,
        crate::domain::packages::types::VersionText,
    >,
    /// The files the workspace governs, sorted: those in its `include`, less those of its local packages.
    pub files: Vec<crate::domain::paths::types::RepoPath>,
    /// The packages it builds from a local path inside its `include`.
    pub local_packages: Vec<LocalPackage>,
}

/// The directory a repository is at, as the packages' directories are written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepositoryText<'a>(pub &'a str);

/// A package that lies in the repository, and its path there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackagePath {
    /// The package.
    pub name: PackageName,
    /// Its directory, relative to the repository's root.
    pub path: crate::domain::paths::types::RepoPath,
}

/// A local `tastemaker.toml`, by its path, and its text.
#[derive(Debug)]
pub struct LocalText {
    /// Where it is.
    pub path: crate::domain::paths::types::RepoPath,
    /// Its text.
    pub text: crate::domain::files::types::FileText,
}

/// What the read of the workspace's own files answers before any definition is read.
#[derive(Debug)]
pub struct Inputs {
    /// The repository's root on disk.
    pub repository: crate::domain::files::types::MachinePath,
    /// The paths the findings are limited to.
    pub within: Vec<crate::domain::paths::types::RepoPath>,
    /// The workspace's directory.
    pub directory: crate::domain::paths::types::RepoPath,
    /// The workspace's `tastemaker.toml`.
    pub file: crate::domain::paths::types::RepoPath,
    /// Its text.
    pub root_text: crate::domain::files::types::FileText,
    /// The lock's text, when there is a lock.
    pub lock_text: LockRead,
    /// The files the workspace governs, its symbolic links among them.
    pub files: Vec<crate::domain::paths::types::RepoPath>,
    /// The symbolic links among `files`, which no rule reads: a link is never followed.
    pub links: Vec<crate::domain::paths::types::RepoPath>,
    /// Its local `tastemaker.toml` files, read.
    pub locals: Vec<LocalText>,
    /// The workspace section.
    pub section: WorkspaceSection,
    /// The workspace file.
    pub workspace: WorkspaceFile,
    /// What is wrong with its variables.
    pub variables: Vec<crate::domain::variables::types::VariableProblem>,
}

/// The files the workspace's `include` names, and its local `tastemaker.toml` files read.
#[derive(Debug)]
pub struct Governed {
    /// The files.
    pub files: Vec<crate::domain::paths::types::RepoPath>,
    /// The local `tastemaker.toml` files, read.
    pub locals: Vec<LocalFile>,
}

/// The file a manifest failure is said of.
#[derive(Clone, Copy, Debug)]
pub struct ErrorFile<'a>(pub &'a str);

/// The workspace's lock as a check reads it: its text, or no lock on disk.
#[derive(Debug)]
pub enum LockRead {
    /// The lock's text.
    Read(crate::domain::files::types::FileText),
    /// There is no lock.
    Absent,
}
