//! The types of the `records` concept: what a run found of each file. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::files::types::FileNumber;
use crate::domain::files::types::FileSize;
use crate::domain::files::types::Nanoseconds;
use crate::domain::stored_facts::types::Bucket;
use crate::domain::stored_facts::types::Digest;
use crate::domain::stored_facts::types::ReadFailure;
use crate::domain::stored_facts::types::{ArenaBytes, FileNotes, StoredPartial};
use crate::domain::stored_facts::types::{KeptWords, PackedWords};
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::{Integer, Span};

/// One value of a row of the answer, typed: a row of a rule's own columns and a finding's row are these, and only the command line
/// writes them as text. `T` holds the text of a file or a text cell: `Box<str>` where the cell is owned, `&str` where a row is lent,
/// and a [`TextRange`] where the text is kept in the text of its rows. A value the engine holds in another kind is one of these with
/// nothing lost: a name is its text; a node and a yes or no are the text they read as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cell<T = Box<str>> {
    /// A file of the repository, by its path relative to the root.
    File(T),
    /// Text: a name, or what a node or a yes or no reads as.
    Text(T),
    /// A whole number.
    Number(Integer),
    /// A stretch of a file, in bytes, as the engine holds it: `NO_SPAN` when the parser gave none.
    Span(Span),
}

/// Where the text of a kept cell is in the text of its rows, in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextRange {
    /// Where it starts.
    pub start: RowsTextAt,
    /// Where it ends.
    pub end: RowsTextAt,
}

/// A place in the text of rows, in bytes from its start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowsTextAt(pub u32);

/// The text of a file or a text cell, lent from where it is kept.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellText<'a>(pub &'a str);

/// The text of the file and text cells of rows, one after the other, owned.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowsText(pub Box<str>);

/// The text of the file and text cells of rows, one after the other, lent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowsTextView<'a>(pub &'a str);

/// The text of the file and text cells of a file's rows, one after the other, as it is written: the buffer a worker keeps the text
/// of its cells in and reuses for the next file, so it is a `String` and not a `Box<str>`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenderedText(pub String);

impl std::ops::Deref for RenderedText {
    type Target = String;

    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for RenderedText {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

impl std::fmt::Write for RenderedText {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.push_str(s);
        Ok(())
    }
}

/// Where each row ends among the cells of the rows, row after row, owned.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RowEnds(pub Vec<u32>);

/// The rule that emitted a row, by its place among the rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedRule(pub u32);

/// The line a row is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedLine(pub u32);

/// A row, by its place among the rows rendered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedRow(pub u32);

/// The character in the line a row is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedColumn(pub u32);

/// What a run kept of one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    /// The exact executable-permission observation used to produce this file outcome.
    pub executable: crate::domain::files::types::ExecutablePermission,
    /// The path, relative to the root.
    pub path: FilePath,
    /// The file's size when it was read.
    pub size: FileSize,
    /// When the file was last written, when it was read.
    pub modified: Nanoseconds,
    /// What its bytes came to.
    pub digest: Digest,
    /// The number the file had in the run that kept this.
    pub number: FileNumber,
    /// What came of the file.
    pub outcome: FileOutcome,
}

/// What came of a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileOutcome {
    /// No language read it.
    Unread {
        /// Why, as the report says it.
        reason: Unreadable,
        /// What the rules that ask for the file gave nonetheless, when a rule did.
        ran: Option<Ran>,
    },
    /// Its rules were evaluated.
    Read(Ran),
}

/// What the rules gave of a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ran {
    /// Its findings as typed cells, the rows of the file in the order they leave in.
    pub rows: FindingRows,
    /// The number the file had when the rules ran on it.
    pub number: FileNumber,
    /// What the file shows other files: what its language noted of it, the relations it is read in part, and what it hands the
    /// rules that read across files. An edit that leaves it as it was leaves what those rules find as it was.
    pub interface: Digest,
    /// What it hands the steps that read across files and the project, for a run that does not read it again.
    pub handed: Handing,
    /// What it hands each rule that reads across files and is handed something of it: an edit that leaves a rule's as it was leaves
    /// what that rule finds as it was.
    pub frontier: Vec<RuleFrontier>,
    /// What its language noted of it and the relations it is read in part come to: an edit that changes them changes what the
    /// project and every rule that reads across files is given.
    pub noted: Digest,
}

/// What a file hands one rule that reads across files, as a digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleFrontier {
    /// The rule, by its place among the rules.
    pub rule: PlacedRule,
    /// What the file hands it.
    pub digest: Digest,
}

/// Where what a file hands the steps that read across files is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Handing {
    /// Made by this run, which read the file.
    Made(Handed),
    /// In the store, as the last run kept it: the file was not read.
    Kept,
}

/// What a file hands the steps that read across files and the project: its words in each bucket, with the numbers that differ from
/// run to run taken out, the text its names made that is in no file, what its language noted of it, and the relations it is read
/// in part with the rules that govern it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Handed {
    /// Independently versioned readiness of demanded per-file named frontiers.
    pub observations: Option<std::sync::Arc<FrontierReadiness>>,
    /// Its words in each bucket, as if it were the first file read by the first worker.
    pub buckets: Vec<HandedWords>,
    /// The text its names made, from the start of its own.
    pub text: ArenaBytes,
    /// What its language noted of it for the project.
    pub notes: HandedNotes,
    /// The rules that govern it, by their place, when a relation of it is read in part.
    pub governing: GoverningRules,
    /// The relations of it read in part.
    pub partials: Vec<StoredPartial>,
}

/// What a file's language noted of it for the project.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum HandedNotes {
    /// It noted this.
    Noted(FileNotes),
    /// It noted nothing.
    #[default]
    Nothing,
}

/// The per-run store snapshot that owns a temporary normalized-word span.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StagingSession(pub u64);

/// A checked byte range in a temporary spool owned by one live ReadStore snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StagedWords {
    /// Foreign or stale sessions are never resolved by another snapshot.
    pub session: StagingSession,
    /// Byte offset from the start of this session's spool.
    pub offset: u64,
    /// Exact packed payload length, with no framing bytes.
    pub length: u64,
}

/// Normalized fresh words are either inline or a descriptor, never a domain-owned file handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordWords {
    /// Memory/discard/custom stores may retain an owned inline payload.
    Inline(PackedWords),
    /// The same run's store snapshot owns these bytes until final publication or drop.
    Staged(StagedWords),
}

impl From<PackedWords> for RecordWords {
    fn from(words: PackedWords) -> Self {
        Self::Inline(words)
    }
}

/// The words a file handed the steps that read across files in one bucket, packed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandedWords {
    /// The bucket.
    pub bucket: Bucket,
    /// Source-bound coverage and root identity for this exact bucket.
    pub proof: FrontierProof,
    /// The words.
    pub words: RecordWords,
}

/// The physical absence scope retained with a per-file frontier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FrontierAffected {
    /// This frontier's containing file is incomplete.
    CurrentFile,
    /// The source reported a repository-wide incomplete relation.
    Repository,
}

/// A complete frontier or the precise physical scopes still uncertain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrontierCoverage {
    /// Every physical input needed by this frontier was complete.
    Complete,
    /// A nonempty, distinct set of affected scopes; validated at admission.
    Partial(Vec<FrontierAffected>),
}

/// Persisted root identity. The containing file and bucket remain part of the
/// enclosing record; a new owner validates this proof against its current rule
/// digest, frontier ordinal, physical file and observed coverage before use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontierProof {
    /// Proof layout version; unknown versions are refused.
    pub version: u32,
    /// Producer rule ordinal in the request that wrote this bucket.
    pub rule: u32,
    /// Original frontier ordinal within that rule.
    pub input: u32,
    /// Current compiled rule identity, compared with the stored rule contract.
    pub rule_digest: Digest,
    /// The exact physical coverage of this frontier.
    pub coverage: FrontierCoverage,
}

/// The rules that govern a file, by their place among the rules.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GoverningRules(pub Vec<u32>);

/// Why no language read a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unreadable {
    /// No linked language reads files like it.
    NoLanguage,
    /// A language reads files like it and failed on this one.
    Failed(ReadFailure),
}

/// A file no language read, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unread {
    /// The file's path, relative to the root.
    pub path: FilePath,
    /// Why it was not read.
    pub reason: Unreadable,
}

/// What a file handed to the cross-file steps in one bucket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kept {
    /// The bucket.
    pub bucket: Bucket,
    /// Proof of this bucket's producer and physical coverage.
    pub proof: FrontierProof,
    /// The words, as the store keeps them.
    pub words: KeptWords,
}

/// Findings as typed cells: every cell one after the other, the text of their file and text cells, where each row ends, and the
/// order the rows leave in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FindingRows {
    /// The text of the file and text cells, one after the other.
    pub text: RowsText,
    /// Every cell, row after row, its text kept in `text`; a cell with no value is `None`.
    pub cells: Vec<Option<Cell<TextRange>>>,
    /// Where each row ends in `cells`.
    pub ends: RowEnds,
    /// The rows, in the order they leave in.
    pub order: Vec<RowPlacing>,
    /// How many rows the file had.
    pub total: RowTotal,
}

/// Where one row of findings is among the rows, and which rule gave it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowPlacing {
    /// The rule that emitted the row.
    pub rule: PlacedRule,
    /// The line it is on.
    pub line: PlacedLine,
    /// The character in the line.
    pub column: PlacedColumn,
    /// The row's place among the rows rendered.
    pub row: PlacedRow,
}

/// Whether a file is as the store kept it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freshness {
    /// The system says nothing of it changed.
    AsItWas,
    /// It changed, or the system cannot tell.
    Changed,
}

/// A count of rows, to be written as a number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowTotal(pub u64);

/// Compact actual readiness beside existing frontier words. Slots are bound to the stable semantic layout identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontierReadiness {
    /// Encoding version; old versions cannot establish completion.
    pub version: crate::domain::observations::types::ObservationReadinessVersion,
    /// Stable ordered producer/output/query/config identity for these bits.
    pub layout: crate::domain::observations::types::ObservationReadinessLayout,
    /// Exact number of logical roots represented.
    pub slots: crate::domain::observations::types::ObservationReadinessCount,
    /// Successful production bits, including actual complete zero-row outputs.
    pub complete: Vec<u64>,
    /// Explicit not-applicable bits; empty storage means no skips, never completion.
    pub skipped: Vec<u64>,
    /// Only explicit failures need owned detail. An unset slot is Unknown, never Complete.
    pub refusals: Vec<FrontierRefusal>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
/// One explicitly unavailable root; ordinary pending slots need no owned reason.
pub struct FrontierRefusal {
    /// Position in the enclosing validated map.
    pub slot: crate::domain::observations::types::ObservationReadinessSlot,
    /// Why production cannot establish complete rows.
    pub reason: FrontierRefusalReason,
}
#[derive(Clone, Debug, PartialEq, Eq)]
/// A typed unavailable state, never an empty successful population.
pub enum FrontierRefusalReason {
    /// A real provider, dependency or retention refusal.
    Unknown(crate::domain::observations::types::ObservationReason),
    /// The selected producer has no executable query.
    NoQuery,
}

/// Canonical read-only retention measurements over portable payloads.
pub mod retention;
