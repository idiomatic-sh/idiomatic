//! The types of the `modules` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages;

/// The path of a manifest, relative to the root of the repository.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ManifestPath(pub Box<str>);

/// A directory, relative to the root of the repository; empty for the root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Directory<'a>(pub &'a str);

/// A directory as a manifest writes it, relative to its own: `./app`, `../util`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Relative<'a>(pub &'a str);

/// A directory of the repository, worked out from the directory of a manifest and a relative one, or from a file's path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Located(pub String);

/// A part of a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Part<'a>(pub &'a str);

/// The path of a module, as `go.mod` writes it: `example.com/lib`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModulePath(pub Box<str>);

/// The version a requirement names, as `go.mod` writes it: `v1.4.0`, a pseudo-version, or `+incompatible`; minimal version selection
/// reads it as the least version the module accepts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModuleVersion(pub Box<str>);

/// What a `replace` sends a module to, as `go.mod` writes it: a directory of the repository when it starts with `.`, another module's
/// path otherwise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModuleReplacementTarget(pub Box<str>);

/// What a `replace` writes after its arrow, as `go.mod` writes it: the module path and its version, or the directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Replacing(pub String);

/// A directory a `go.work` uses, as it writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Member(pub Box<str>);

/// What the tool said of a manifest it could not read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Reason(pub Box<str>);

/// Why a relation is read only in part: a manifest that was refused, with what the tool said, or an import that names a package of
/// the module no directory is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Cause(pub Box<str>);

/// A requirement of a module on another, as `go.mod` writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Requirement {
    /// The path of the module required.
    pub path: ModulePath,
    /// The version it requires, as written; empty when the line writes none, which `go` refuses.
    pub version: ModuleVersion,
    /// Whether the module is needed by a requirement of the module's own and not by its code (`// indirect`).
    pub indirect: Indirectness,
}

/// Whether a requirement is one the code of the module does not make itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Indirectness {
    /// Another module's requirement brings it.
    Indirect,
    /// The module's own code needs it.
    Direct,
}

/// A replacement of a module by another, as `go.mod` writes it: the module and the version of it replaced, and where its code is
/// instead, with the version taken there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Replacement {
    /// The path of the module replaced.
    pub from: ModulePath,
    /// The version of it replaced; empty when every version is (Go Modules Reference, "replace directive").
    pub version: ModuleVersion,
    /// What replaces it.
    pub to: ModuleReplacementTarget,
    /// The version of the module that replaces it; empty when a directory does, which takes none.
    pub to_version: ModuleVersion,
}

/// What one manifest declares.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Declared {
    /// The path of the module, for a `go.mod`; none for a `go.work`.
    pub module: Option<ModulePath>,
    /// The modules it requires.
    pub requirements: Vec<Requirement>,
    /// The modules it replaces.
    pub replacements: Vec<Replacement>,
    /// The directories a workspace uses, as it writes them.
    pub uses: Vec<Member>,
}

/// A manifest of the repository: where it is and what it declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Manifest {
    /// The manifest.
    pub manifest: ManifestPath,
    /// What it declares.
    pub declared: Declared,
}

/// A manifest that could not be read, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Refusal {
    /// The manifest.
    pub manifest: ManifestPath,
    /// What the tool said.
    pub reason: Reason,
}

/// The manifests of a repository, in the order of their paths, and the ones that could not be read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Modules {
    /// The manifests that were read.
    pub manifests: Vec<Manifest>,
    /// The manifests the tool refused.
    pub refused: Vec<Refusal>,
}

/// A requirement of a module the run lists, as the relation `package_dependency` writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Wiring {
    /// The file of the `go.mod` of the module that requires.
    pub source: languages::FileId,
    /// The file of the `go.mod` of the module required; `NO_FILE` when the repository does not hold it.
    pub target: languages::FileId,
    /// Where the manifest says the module comes from: a directory a `replace` sends it to, or else a module proxy.
    pub origin: languages::DependencyOrigin,
    /// The path of the module required, as the requirement writes it.
    pub path: ModulePath,
    /// The version it requires, as the requirement writes it.
    pub version: ModuleVersion,
    /// The `replace` that applies to the requirement, when one does.
    pub replacement: Option<Replacement>,
}

/// A workspace and one of its members, as the relation `workspace_member` writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WorkspaceMembership {
    /// The file of the `go.work`.
    pub workspace: languages::FileId,
    /// The file of the `go.mod` of the member.
    pub module: languages::FileId,
}

/// A file of the run and the manifest of its module, as the relation `file_package` writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FilePlacement {
    /// The file.
    pub path: languages::FileId,
    /// The file of the `go.mod` of its module.
    pub module: languages::FileId,
}

/// What a reader of the project writes its rows from: the rows themselves, and the manifests that were refused, for which the relation is
/// read only in part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Wired<Row> {
    /// The rows.
    pub rows: Vec<Row>,
    /// Why each manifest the tool refused is missing from them.
    pub refused: Vec<Cause>,
}

/// Which manifest a path is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Manifestness {
    /// A `go.mod`.
    Module,
    /// A `go.work`.
    Workspace,
    /// Neither.
    Other,
}

/// A requirement of a module's `go.mod`, as its tree holds it: the line that writes it, the word naming the module, the word of its
/// version when it has one, the element whose requirements it is counted among (its block, or the file for a line of its own) and its
/// place among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RequiredLine {
    /// The word naming the module.
    pub path: languages::Position,
    /// The word of its version, when the line writes one.
    pub version: Option<languages::Position>,
    /// The block the line is in, or the file for a line of its own.
    pub owner: languages::Position,
    /// Its place among the requirements of the owner, from zero.
    pub ordinal: languages::Integer,
}

/// A mapping of a `go.mod`: how it is written and what holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ModuleMapping {
    /// How it is written: the file is implied by its lines, a `require` block is opened by its own header.
    pub kind: languages::MappingKind,
    /// The file for a block; the file holds itself.
    pub owner: languages::Position,
}

/// The verb a `go.mod` line or block starts with, as far as a requirement is concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verb {
    /// `require`: the line, or each line of the block, names a module the module requires.
    Require,
    /// Any other directive.
    Other,
}
