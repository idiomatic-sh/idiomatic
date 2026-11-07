//! The types of the `proofs` concept: plain data, public fields and derives, with no constructor and no inherent impl.
//! A proof says that one version of one definition holds: its query runs, it explains itself, and its scenarios pass.

use crate::domain::files::types::FileText;
use crate::domain::packages::types::{PackageRoot, Release};

/// One file read by a version proof, or one version's directory, relative to its package.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProofFile {
    /// The package containing it.
    pub directory: PackageRoot,
    /// Its package-relative path.
    pub path: ProofPath,
}

/// A package-relative proof input path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProofPath(pub Box<str>);

/// The ordered byte slices hashed to identify a proof.
#[derive(Debug)]
pub struct ProofParts<'a>(pub Vec<&'a [u8]>);

/// What a proof's inputs take from one definition file: the shared releases its spec composes and the `tastemaker.toml`
/// files its scenarios write. It follows from the file's text alone, and is read again only when the file changed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DefinitionReading {
    /// The shared releases the file's composition names; none for a rule or a template.
    pub composed: Vec<Release>,
    /// The `tastemaker.toml` files its scenarios write, in order.
    pub written: Vec<WrittenFile>,
}

/// A `tastemaker.toml` a scenario writes: where in the scenario's project, and its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrittenFile {
    /// Its path in the scenario's project.
    pub path: crate::domain::paths::types::RepoPath,
    /// Its text.
    pub text: FileText,
}

/// What is wrong with a version, in a sentence.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Problem(pub Box<str>);

/// What proving a version answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Proof {
    /// It holds.
    Proven,
    /// It does not, and why.
    Failed(Vec<Problem>),
}

/// One version of one definition to prove: its id and the files its digest reads.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Subject {
    /// `package/kind/name@version`.
    pub id: SubjectId,
    /// Its directory within its package.
    pub directory: SubjectDirectory,
}

/// What a version another reads through is named in its digest: `kind/package/name@version`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UseText(pub Box<str>);

/// The name of one part of a node's digest: a file's path, or the text of a version used; the node's own is empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeName<'a>(pub &'a str);

/// One part of a node's digest: its name, and the digest of what it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodePart<'a> {
    /// Its name.
    pub name: NodeName<'a>,
    /// The digest of what it names.
    pub digest: &'a crate::domain::digests::types::Digest,
}

/// The paths a version's stamp is taken over: the files of its directory, and every directory of it, its own first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VersionPaths<'a> {
    /// The files.
    pub files: Vec<crate::domain::paths::types::PathText<'a>>,
    /// The directories.
    pub directories: Vec<crate::domain::paths::types::PathText<'a>>,
}

/// Whether what a kept proof rests on still holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grounds {
    /// It does.
    Holds,
    /// It moved.
    Moved,
}

/// The id of one version of one definition: `package/kind/name@version`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubjectId(pub Box<str>);

/// The directory of one version of one definition, within its package.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubjectDirectory(pub Box<str>);

/// The name of the rule a scenario proves: its findings are written without the rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoundOwn<'a>(pub &'a str);

/// What names a definition in a message: `rule `name``, or `rule `name` of spec `spec``.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionLabel(pub String);

/// One scenario of a definition and what it expects: its id, and whether the check finds anything.
#[derive(Clone, Copy, Debug)]
pub struct JudgedScenario<'a> {
    /// The scenario.
    pub id: &'a crate::domain::scenarios::types::ScenarioId,
    /// Whether it expects findings.
    pub verdict: crate::domain::scenarios::types::Expectation,
}

/// Whether ryo runs a rule's query while a scenario is proved.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Running {
    /// It runs.
    Runs,
    /// It is refused.
    Refused,
}

/// A version validation proved held: the release it is, the digest it held under, and every other version it reads
/// through. Being kept is the proof: a version that fails is not kept.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProvenVersion {
    /// The version.
    pub release: Release,
    /// The stamp of its directory's files and its uses', all told, when it held: while it does not move, no file is read
    /// again.
    pub stamp: crate::domain::files::types::Stamp,
    /// The digest of its directory's files, and of each version it reads through's.
    pub digest: crate::domain::digests::types::Digest,
    /// Every other version it reads through: those its spec composes and its scenarios' own `tastemaker.toml` files
    /// name, and those they read through in turn.
    pub uses: Vec<Release>,
}

/// The versions a validation proved held, as one run keeps them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProvenVersions(pub Vec<ProvenVersion>);
