//! Named text and yes/no values of the registered and observed documents.

/// A rule identifier as a document spells it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuleName(pub String);

/// A case name as a document spells it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CaseName(pub String);

/// A lowercase SHA256 digest or another sealed identity, as written.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sha256Text(pub String);

/// A relative or absolute path as a document spells it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PathText(pub String);

/// Why a document records a failure or a limitation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReasonText(pub String);

/// A free-form sub-document compared only for equality, held in canonical sorted-key form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalJson(pub String);

/// The schema label an observation names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaName(pub String);

/// A yes or no a document records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YesNo {
    /// The document says yes.
    Yes,
    /// The document says no.
    No,
}

impl From<bool> for YesNo {
    fn from(value: bool) -> Self {
        if value { Self::Yes } else { Self::No }
    }
}

/// Which contestant an observation row belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ContestantSide {
    /// The pinned reference tool.
    Reference,
    /// The tool under measurement.
    Candidate,
}

/// The exit status or signal number of a process the reference ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitStatus(pub i64);

/// The terminating signal of a captured process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signal {
    /// The process was not signalled.
    Absent,
    /// The process was ended by this signal.
    Number(ExitStatus),
}

/// An identity a record may leave out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pin {
    /// The record binds no such identity.
    Absent,
    /// The record binds this identity.
    Sha256(Sha256Text),
}

/// The digest of the declared file metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileMetadataDigest {
    /// The case declares file metadata, under this digest.
    Declared(Sha256Text),
    /// The case declares no file metadata.
    Undeclared,
}

impl From<Option<Sha256Text>> for FileMetadataDigest {
    fn from(digest: Option<Sha256Text>) -> Self {
        digest.map_or(Self::Undeclared, Self::Declared)
    }
}

/// The digest of the declared project inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectInputsDigest {
    /// The case declares project inputs, under this digest.
    Declared(Sha256Text),
    /// The case declares no project inputs.
    Undeclared,
}

impl From<Option<Sha256Text>> for ProjectInputsDigest {
    fn from(digest: Option<Sha256Text>) -> Self {
        digest.map_or(Self::Undeclared, Self::Declared)
    }
}
