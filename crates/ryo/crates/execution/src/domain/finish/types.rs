//! Borrowed publication storage belongs to the execution invocation.
use ryo_language::domain::symbols::types::Symbols;

/// The exact failed portability invariant; control flow never compares diagnostic text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationReason {
    ReservedArena,
    CrossNameMismatch,
    PlaceUnrepresentable,
    PlaceMissing,
    SourceTextMissing,
    StartUnrepresentable,
    EndUnrepresentable,
    LocalRangeMissing,
    LocalHashMismatch,
    LocalTextInvalid,
    NormalizedIdentityMismatch,
    InheritedArenaUnrepresentable,
    InheritedRangeMissing,
    InheritedTextMismatch,
    PortablePlaceLogFull,
    ProducerSymbolUnescaped,
    CrossPlaceUnescaped,
    EscapedPlaceUnrepresentable,
    EscapedPlaceMissing,
    EscapedArenaUnstable,
}

use ryo_relations::domain::relations::types::{FileId, NamePlace, WorkerNumber};
use ryo_values::domain::files::types::SourceView;
/// Where the names of a file's rows are read, and who they leave as: the file's names and bytes, the file, and the worker that
/// keeps the text of a name made while evaluating.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Leaving<'a> {
    /// The names of the file.
    pub symbols: &'a Symbols,
    /// The bytes of the file.
    pub source: SourceView<'a>,
    /// The file.
    pub file: FileId,
    /// The worker.
    pub worker: WorkerNumber,
    /// Whether a name leaves with its text, for steps that read the text after the files were read.
    pub text: Leaves,
}

/// Whether a name that leaves its file takes its text with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Leaves {
    /// The text is moved to the worker's own, so steps that read the text after the files were read can.
    WithText,
    /// The text stays in the file; only the tests of `escape` leave a name so, a run always takes the text.
    #[cfg(test)]
    WithoutText,
}

/// Where the text and the places of the names that left are kept: the worker's own text, and the places of one table.
#[derive(Debug)]
pub(crate) struct Retained<'a> {
    /// The text the worker made, which it keeps for the whole run.
    pub arena: Arena<'a>,
    /// Where the text of each name that left is.
    pub places: &'a mut Vec<NamePlace>,
}

/// The text a worker made and keeps for the whole run, which a name that leaves its file may be moved to the end of.
#[derive(Debug)]
pub(crate) struct Arena<'a>(pub &'a mut Vec<u8>);
