//! Portable reasons for known relation absence.

/// Why a language does not answer a relation: a reason the engine can list, not a sentence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AbsenceCause {
    /// The language has no such construct.
    NoSuchConstruct,
    /// The language is a format of data, which has no such construct of code.
    NotCode,
    /// The language is a language of code, which has no keyed documents.
    NotAKeyedDocument,
    /// The language is not a language of documents, which have no prose.
    NotADocument,
    /// The language has the construct and has not written the reader of it yet: a stage of a language being written, never an answer a
    /// finished language gives for a construct it has.
    NotYetRead,
    /// The language has no project manifests.
    NoManifests,
}
