//! The types of the `stores` concept: what a lint keeps for the next run, one table per kind of thing, a row only for what is current
//! and correct. Only the findings a run shows and the digests that decide what the next can use again are kept; nothing a run can
//! make again cheaply is. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::files::types::{FileSize, Nanoseconds};
use crate::domain::descriptions::types::ColumnName;
use crate::domain::records::types::{
    Cell, GoverningRules, PlacedColumn, PlacedLine, PlacedRule, Unreadable,
};
use crate::domain::runs::types::{ManifestRefused, RuleId, Skipped};
use crate::domain::stored_facts::types::PackedWords;
use crate::domain::stored_facts::types::{ArenaBytes, Bucket, Digest, FileNotes, StoredPartial};
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::project_context::types::EntryObservation;

/// What a lint keeps for the next run: its rules, its files, what they showed and what could not be read or decided.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stored {
    /// The rules the findings were made with.
    pub rules: Vec<StoredRule>,
    /// The files, in the order of their paths, each numbered by its place here.
    pub files: Vec<StoredFile>,
    /// The findings: those of each file, in the order of the files, then those of the rules that read across files.
    pub findings: Vec<StoredFinding>,
    /// The files that were listed and that no language read, with why.
    pub unread: Vec<StoredUnread>,
    /// The manifests the languages' tools refused.
    pub refused: Vec<ManifestRefused>,
    /// What the rules could not decide, because a relation they trust the absence of rows of was read only in part.
    pub unknowns: Vec<StoredUnknown>,
    /// Rules explicitly skipped because a file's syntax was rejected.
    pub skipped: Vec<Skipped>,
    /// What each file hands each rule that reads across files, one row for each file and rule it hands something.
    pub frontiers: Vec<StoredFrontier>,
    /// The files the project's lookup read beyond the manifests the run handed it, in the order of their paths: configurations above
    /// the root (`../` paths) or in it, and manifests it read from the disk. A warm run makes the rules that read the project again
    /// only when one of them is gone or its digest changed. Their interface and notes are the zero digest: no language reads them as
    /// a file.
    pub consulted: Vec<StoredFile>,
    /// The files the lookup asked for and found none of, in the order of their paths: a warm run makes the rules that read the
    /// project again when one of them is there now.
    pub consulted_absent: Vec<FilePath>,
    /// Exact project-entry states used to answer imported-root membership.
    pub project_observations: Vec<EntryObservation>,
    /// What each file handed the steps that read across files and the project: read only by a run that makes those steps again or
    /// keeps a store of its own.
    pub handed: StoredHanded,
}

/// What a file hands one rule that reads across files, as a digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredFrontier {
    /// The file.
    pub file: StoredFileNumber,
    /// The rule, by its place among the rules.
    pub rule: PlacedRule,
    /// What the file hands it.
    pub digest: Digest,
}

/// Which buckets of what the files handed a run reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BucketsWanted {
    /// Every bucket.
    All,
    /// These buckets, in order.
    These(Vec<Bucket>),
}

/// What a store keeps of what the files handed the steps that read across files besides the rows a run gives it: the buckets it
/// keeps as they were, and the files used again, whose rows of the other buckets it moves to their new number.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptRows {
    /// Fresh staged payloads, resolved only by the same run's store snapshot during writing.
    pub staged: Vec<StagedRow>,
    /// The buckets whose words are kept as they were, in order.
    pub unchanged: Vec<Bucket>,
    /// Each file the run used again, in the order of both its numbers.
    pub file_again: Vec<FileAgain>,
}

/// Fresh normalized words that will be streamed into the unchanged persistent bucket format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StagedRow {
    /// The file's final number in this run's store.
    pub file: StoredFileNumber,
    /// Its destination bucket.
    pub bucket: Bucket,
    /// Original fresh-row position among both inline and staged rows, preserving tie order.
    pub ordinal: u64,
    /// Plain descriptor; no path or IO resource crosses into domain data.
    pub words: crate::domain::records::types::StagedWords,
}

/// A file a run used again: its number in the store kept before, and its number now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileAgain {
    /// Its number in the store kept before.
    pub was: StoredFileNumber,
    /// Its number now.
    pub now: StoredFileNumber,
}

/// What each file handed the steps that read across files and the project, in tables of their own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoredHanded(pub HandedRows<StoredWords>);

impl std::ops::Deref for StoredHanded {
    type Target = HandedRows<StoredWords>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for StoredHanded {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// One snapshot's read representation; old words need not be retained as payload bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReadHanded(pub HandedRows<ReadWords>);

impl std::ops::Deref for ReadHanded {
    type Target = HandedRows<ReadWords>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ReadHanded {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Rest tables share their unchanged representation across persistent writes and snapshot reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandedRows<W> {
    /// Per-file named-root readiness, in stored file order.
    pub observations: Vec<StoredFrontierReadiness>,
    /// One typed source/coverage proof for each retained physical bucket row.
    /// This metadata stays beside the existing packed payload sections.
    pub proofs: Vec<StoredFrontierProof>,
    /// What each file handed the steps that read across files, one row for each file and bucket.
    pub frontier: Vec<W>,
    /// The text the names of each file made that is in no file, one row for each file that made some.
    pub texts: Vec<StoredText>,
    /// What the language of each file noted of it for the project, one row for each file it noted something of.
    pub notes: Vec<StoredNotes>,
    /// The relations of each file read in part, with the rules that govern it, one row for each file read in part.
    pub covered: Vec<StoredCovered>,
}

impl<W> Default for HandedRows<W> {
    fn default() -> Self {
        Self {
            observations: Vec::new(),
            proofs: Vec::new(),
            frontier: Vec::new(),
            texts: Vec::new(),
            notes: Vec::new(),
            covered: Vec::new(),
        }
    }
}

/// Versioned provenance of one file/bucket pair, validated against the
/// prepared rule and file identity before a warm row is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredFrontierProof {
    pub file: StoredFileNumber,
    pub bucket: Bucket,
    pub proof: crate::domain::records::types::FrontierProof,
}

/// Read-only row metadata, retaining physical row order and duplicates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadWords {
    /// File number in the immutable old snapshot.
    pub file: StoredFileNumber,
    /// Original bucket identity.
    pub bucket: Bucket,
    /// Source-bound root and physical coverage beside the retained payload.
    pub proof: crate::domain::records::types::FrontierProof,
    /// Inline custom-store bytes or a lazy old-file descriptor.
    pub words: ReadWordPayload,
}

/// Read payload ownership is separate from the unchanged persistent write contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadWordPayload {
    /// In-memory/custom adapters may retain their existing representation.
    Inline(PackedWords),
    /// This descriptor is resolved only by the ReadStore that issued it.
    Old(OldWords),
}

/// Process-local identity of one immutable opened store; never a pathname or file handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoreSnapshot(pub u64);

/// Validated old row framing metadata. No resource, bytes, or IO behavior belongs here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OldWords {
    /// Reader identity, checked before touching its file.
    pub snapshot: StoreSnapshot,
    /// Bucket and file encoded by this row.
    pub bucket: Bucket,
    /// File number encoded by this row.
    pub file: StoredFileNumber,
    /// Start of the row's file-number and payload-length header.
    pub header: u64,
    /// Start of its packed bytes in the opened snapshot.
    pub offset: u64,
    /// Exact packed byte count.
    pub length: u64,
}

/// The words a file handed the steps that read across files in one bucket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredWords {
    /// The file.
    pub file: StoredFileNumber,
    /// The bucket.
    pub bucket: Bucket,
    /// Source-bound root and physical coverage beside the retained payload.
    pub proof: crate::domain::records::types::FrontierProof,
    /// The words, as if the file were the first read by the first worker, packed.
    pub words: PackedWords,
}

/// The text a file's names made that is in no file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredText {
    /// The file.
    pub file: StoredFileNumber,
    /// The text.
    pub text: ArenaBytes,
}

/// What the language of a file noted of it for the project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredNotes {
    /// The file.
    pub file: StoredFileNumber,
    /// The language that wrote them, and the notes.
    pub notes: FileNotes,
}

/// A file read in part, with the rules that govern it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredCovered {
    /// The file.
    pub file: StoredFileNumber,
    /// The rules that govern it.
    pub governing: GoverningRules,
    /// The relations of it read in part.
    pub partials: Vec<StoredPartial>,
}

/// Whether a rule reads across files: its findings depend on more than the file they are in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crossing {
    /// Its findings in a file come of that file alone.
    Within,
    /// It reads across files, or what the project writes.
    Across,
}

/// A rule the findings were made with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRule {
    /// Its id.
    pub rule: RuleId,
    /// What its query, its parameters, its files and what the host declared come to: a rule whose digest changed is made again.
    pub digest: Digest,
    /// Whether it reads across files.
    pub crossing: Crossing,
}

/// A file a run read or used again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredFile {
    /// The exact executable-permission observation used to produce this file outcome.
    pub executable: crate::domain::files::types::ExecutablePermission,
    /// Its path, relative to the root.
    pub path: FilePath,
    /// What its bytes came to.
    pub digest: Digest,
    /// What it shows other files came to: what its language noted of it, the relations it is read in part, and what it hands the
    /// rules that read across files.
    pub interface: Digest,
    /// Its size when it was read: with `modified`, what says the file is as it was without reading it.
    pub size: FileSize,
    /// When it was last written, when it was read.
    pub modified: Nanoseconds,
    /// What its language noted of it and the relations it is read in part come to.
    pub noted: Digest,
}

/// A file, by its place among the stored files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StoredFileNumber(pub u32);

/// Which visit made a finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindingMade {
    /// The visit of the file at this place among the stored files.
    InFile(StoredFileNumber),
    /// The rules that read across files, once every file was visited.
    AcrossFiles,
}

/// Where in its subject a finding is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindingPlace {
    /// At this line and character.
    At(StoredPlace),
    /// Nowhere in particular: the finding is about its subject as a whole.
    Unplaced,
}

/// Where in its file a finding is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredPlace {
    /// The line.
    pub line: PlacedLine,
    /// The character in the line.
    pub column: PlacedColumn,
}

/// A finding a run showed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredFinding {
    /// The rule that found it.
    pub rule: RuleId,
    /// Which visit made it.
    pub made: FindingMade,
    /// Where it is in its subject.
    pub place: FindingPlace,
    /// What it is about.
    pub subject: StoredCell,
    /// What tells it apart from the rule's others: the value of each of the rule's key columns, in order.
    pub key: Vec<StoredCell>,
    /// Its columns, in the order the rule writes them.
    pub values: Vec<StoredValue>,
}

/// One column of a finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredValue {
    /// Its heading.
    pub column: ColumnName,
    /// Its value.
    pub value: StoredCell,
}

/// A cell of a finding as the store keeps it: its value, or nothing when the value reads as nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoredCell {
    /// The value.
    Value(Cell),
    /// The value reads as nothing.
    Nothing,
}

impl From<Option<Cell>> for StoredCell {
    fn from(cell: Option<Cell>) -> Self {
        cell.map_or(Self::Nothing, Self::Value)
    }
}

impl<'a> From<&'a StoredCell> for Option<&'a Cell> {
    fn from(cell: &'a StoredCell) -> Self {
        match cell {
            StoredCell::Value(value) => Some(value),
            StoredCell::Nothing => None,
        }
    }
}

/// A file no language read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredUnread {
    /// The file.
    pub file: StoredFileNumber,
    /// Why: no linked language reads such a file, or what the language said when it failed.
    pub reason: Unreadable,
}

/// What a rule could not decide, as shown.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnknownText(pub Box<str>);

/// Something a rule could not decide, and the file that is the reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredUnknown {
    /// The rule.
    pub rule: RuleId,
    /// The file whose relation is read only in part.
    pub path: FilePath,
    /// What is partial there and why.
    pub message: UnknownText,
}

/// One existing stored file's named frontier readiness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredFrontierReadiness {
    pub file: StoredFileNumber,
    pub root: std::sync::Arc<crate::domain::records::types::FrontierReadiness>,
}

/// Which rules that read across files a run makes again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MadeAgain {
    /// None: what they found is as the store kept it.
    Nothing,
    /// Every one: what a file shows the project or every rule changed, or files were added or removed.
    All,
    /// These, by their place, in order: what the changed files hand them changed, and nothing else did.
    These(Vec<crate::domain::records::types::PlacedRule>),
}

/// How much of what the last run kept a run uses again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reuse {
    /// Nothing: every file is read and every rule run.
    Whole,
    /// The files as they were, and the findings of the rules that read across files.
    Again,
    /// The files as they were, and what they handed the steps that read across files, which are made again for the rules named;
    /// the others' findings are as the store kept them.
    Cross(MadeAgain),
}

/// Canonical read-only retention measurements over portable payloads.
pub mod retention;
