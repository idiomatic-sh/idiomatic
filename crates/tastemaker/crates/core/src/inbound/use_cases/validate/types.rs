//! The types of the `validate` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::files::types::MachinePath;
use crate::domain::proofs::types::{Proof, Subject};

/// What `validate` is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidateRequest {
    /// Where to look for the workspace.
    pub start: MachinePath,
    /// Only the versions whose id holds one of these texts; none means every version.
    pub only: Vec<VersionFilter>,
}

/// Whether a proof was kept from an earlier run or made by this one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proving {
    /// Its digest was known: a lookup.
    Kept,
    /// It was proved now.
    Made,
}

/// One version's proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proved {
    /// The version.
    pub subject: Subject,
    /// Its proof.
    pub proof: Proof,
    /// Kept or made.
    pub proving: Proving,
}

/// What `validate` answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidateReport {
    /// Every version's proof, in order.
    pub proved: Vec<Proved>,
}

/// A text a version's id must hold to be proved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionFilter(pub Box<str>);

/// One version to prove, with its files read.
pub(crate) struct Version {
    /// The version, as a message names it.
    pub subject: Subject,
    /// Its release.
    pub release: crate::domain::packages::types::Release,
    /// The directory of its package.
    pub package: crate::domain::packages::types::PackageRoot,
    /// Its definition files, read when it is proved.
    pub files: Vec<crate::outbound::packages::types::PackageFile>,
}

/// One file of a version, by its path in its package, and its text.
pub(crate) struct VersionFile {
    /// Its path in its package.
    pub path: crate::outbound::packages::types::PackageFile,
    /// Its text.
    pub text: crate::domain::files::types::FileText,
}

/// What places the definition a scenario proves when its files bring no `tastemaker.toml`: the version proven, and how
/// its definition is placed.
pub(crate) struct FixturePlacing<'a> {
    /// The version proven.
    pub version: &'a Version,
    /// How its definition is placed.
    pub place: FixturePlace,
}

/// How a fixture without a `tastemaker.toml` of its own places what it proves.
pub(crate) enum FixturePlace {
    /// A shared definition: an entry of the root `tastemaker.toml` naming it.
    Shared {
        /// Its kind, package and name.
        placed: crate::domain::packages::types::PlacedDefinition,
    },
    /// A definition a spec owns: a wrapper of the spec, holding only it, applied at the project's root.
    Owned {
        /// Rule or template.
        kind: crate::domain::packages::types::DefinitionKind,
        /// Its name.
        name: crate::domain::packages::types::DefinitionName,
        /// Its directory, in the package.
        directory: crate::outbound::packages::types::PackageFile,
    },
}

/// A scenario's project, checked: the report its check gave, which the verdict judges against what the scenario
/// expects; or the problem that kept it from being checked.
pub(crate) enum ProjectJudged {
    /// The project was checked.
    Checked(crate::domain::findings::types::Report),
    /// It could not be: the problem.
    Unchecked(crate::domain::proofs::types::Problem),
}

/// One file of a scenario's project: its text, and the directory of the asset it is written in.
pub(crate) struct Materialized {
    /// Its text.
    pub text: crate::domain::files::types::FileText,
    /// The file of the asset it is written in, in its package.
    pub origin: crate::outbound::packages::types::PackageFile,
}

/// What a run learns of each version's digest: each version's own files' digest, by its directory, and its stamp; the
/// versions each
/// names and each uses, those named in turn included; the digest of each, its uses' included; the versions each package
/// offers; the versions whose uses were read this run; and where the packages only a scenario's own `tastemaker.toml`
/// names are.
#[derive(Default)]
pub(crate) struct Digesting {
    /// Each version's own files' digest, by its directory.
    pub contents: std::collections::BTreeMap<
        crate::domain::proofs::types::ProofFile,
        crate::domain::digests::types::Digest,
    >,
    /// The versions each version's definitions name, read this run.
    pub direct: std::collections::BTreeMap<
        crate::domain::packages::types::Release,
        Vec<crate::domain::packages::types::Release>,
    >,
    /// The versions each version uses: those it names, and those they use in turn.
    pub uses: std::collections::BTreeMap<
        crate::domain::packages::types::Release,
        Vec<crate::domain::packages::types::Release>,
    >,
    /// Each version's digest, its uses' included.
    pub done: std::collections::BTreeMap<
        crate::domain::packages::types::Release,
        crate::domain::digests::types::Digest,
    >,
    /// Each version's stamp and its uses', all told, as a row keeps it.
    pub kept: std::collections::BTreeMap<
        crate::domain::packages::types::Release,
        crate::domain::files::types::Stamp,
    >,
    /// Each version's stamp, taken this run.
    pub stamps: std::collections::BTreeMap<
        crate::domain::packages::types::Release,
        crate::domain::files::types::Stamp,
    >,
    /// The versions each package offers of each of its definitions, by its directory.
    pub offered: std::collections::BTreeMap<
        crate::domain::packages::types::PackageRoot,
        std::collections::BTreeMap<
            crate::domain::packages::types::DefinitionKey,
            Vec<crate::domain::packages::types::VersionText>,
        >,
    >,
    /// The versions whose uses were read from their definitions this run.
    pub resolved: std::collections::BTreeSet<crate::domain::packages::types::Release>,
    /// Where each package a scenario's own `tastemaker.toml` names is.
    pub located: std::collections::HashMap<
        crate::domain::packages::types::PackageName,
        crate::domain::packages::types::PackageRoot,
    >,
}

/// A version a run answers for, the key of its proof, and the proof the store holds for that key, if any.
pub(crate) struct AskedProof {
    /// The version.
    pub version: Version,
    /// The key of its proof.
    pub key: crate::domain::digests::types::Digest,
    /// The proof held for that key.
    pub held: Option<Proof>,
}

/// The definitions the projects of a run read, each by its kind, package and file, as a project reads it or why it does
/// not: the projects of many scenarios read the same definitions, so each is read and parsed once a run.
#[derive(Debug, Default)]
pub(crate) struct Parses(
    pub  std::sync::Mutex<
        std::collections::HashMap<
            ParseKey,
            std::sync::Arc<
                Result<
                    crate::domain::loading::types::Parsed,
                    crate::domain::proofs::types::Problem,
                >,
            >,
        >,
    >,
);

/// A definition file a project reads: its kind, its package and the file.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ParseKey {
    /// What it is.
    pub kind: crate::domain::packages::types::DefinitionKind,
    /// Its package.
    pub package: crate::domain::packages::types::PackageRoot,
    /// The file.
    pub file: crate::domain::loading::types::DefinitionFile,
}

/// The package manifests a run reads, by directory, or why one does not read: the workspace and every scenario's project
/// resolve the same packages, so each is read and parsed once a run.
#[derive(Debug, Default)]
pub(crate) struct Manifests(
    pub  std::sync::Mutex<
        std::collections::HashMap<
            crate::domain::packages::types::PackageRoot,
            std::sync::Arc<Result<crate::domain::packages::types::PackageManifest, Unread>>,
        >,
    >,
);

/// Why a package's manifest does not read: its file, or its text.
#[derive(Clone, Debug)]
pub(crate) enum Unread {
    /// The file does not read.
    File(crate::outbound::packages::errors::PackagesError),
    /// The text does not parse.
    Text(crate::outbound::manifest::errors::ManifestError),
}

/// The versions each package offers of a definition, by the package's directory and the definition's directory in it,
/// or why they cannot be listed: every scenario's project asks for the same, so each is listed once a run.
#[derive(Debug, Default)]
pub(crate) struct Offers(
    pub  std::sync::Mutex<
        std::collections::HashMap<
            (
                crate::domain::packages::types::PackageRoot,
                crate::outbound::packages::types::PackageFile,
            ),
            Result<
                crate::outbound::packages::types::Versions,
                crate::outbound::packages::errors::PackagesError,
            >,
        >,
    >,
);

/// ryo's verdicts on the queries of a run, by text: one verdict a text.
#[derive(Debug, Default)]
pub(crate) struct Verdicts(
    pub  std::sync::Mutex<
        std::collections::HashMap<
            crate::domain::definitions::types::QueryText,
            crate::outbound::analysis::types::QueryVerdict,
        >,
    >,
);

/// The wrapper packages a run made, by what they hold: each made once, by the first scenario that asks for it, and held
/// in memory by the packages port, never written to a disk.
#[derive(Debug, Default)]
pub(crate) struct Wrappers(
    pub std::sync::Mutex<std::collections::HashMap<WrapperKey, std::sync::Arc<Wrapped>>>,
);

/// One wrapper package: made once, by the first scenario that asks for it, and the directory the packages port holds it
/// at; or why it could not be made.
#[derive(Debug, Default)]
pub(crate) struct Wrapped(
    pub  std::sync::OnceLock<
        Result<crate::domain::packages::types::PackageRoot, crate::domain::proofs::types::Problem>,
    >,
);

/// What a wrapper package holds: the package it wraps, the directory of the definition it holds, and the text of its own
/// definition file.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct WrapperKey {
    /// The package it wraps.
    pub package: crate::domain::packages::types::PackageRoot,
    /// The directory of the definition it holds.
    pub directory: crate::outbound::packages::types::PackageFile,
    /// The text of its own definition file.
    pub owner: crate::domain::files::types::FileText,
}

/// The five caches a run keeps, lent.
#[cfg(test)]
#[derive(Debug)]
pub(crate) struct Caches<'a> {
    /// The definitions read.
    pub parses: &'a Parses,
    /// The package manifests read.
    pub manifests: &'a Manifests,
    /// The versions listed.
    pub offers: &'a Offers,
    /// ryo's verdicts.
    pub verdicts: &'a Verdicts,
    /// The wrappers made.
    pub wrappers: &'a Wrappers,
}
