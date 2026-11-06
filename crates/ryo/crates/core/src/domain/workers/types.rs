//! The types of the `workers` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::finish::types::LineStarts;
use crate::domain::render::types::{Answering, Rendered};
use crate::domain::visit::types::{Keeping, Noting, RoleRead};
use ryo_execution::inbound::use_cases::execution::types::{PreparedVocabulary, RowsFed};
use ryo_execution::inbound::use_cases::metadata::interface::Needs;
use ryo_execution::inbound::use_cases::metadata::interface::{Covered, Partial, Unknown};
use ryo_execution::inbound::use_cases::metadata::interface::{Mask, PlacedScopes};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{NamePlace, Relation, Words, WorkerNumber};
use ryo_relations::domain::terms::types::UnknownName;
use ryo_values::domain::files::types::{FileBytes, Hits};
use ryo_values::domain::records::types::{Kept, Record, RowTotal, Unread};
use ryo_values::domain::runs::types::MemoryBytes;
use ryo_values::domain::runs::types::RuleApplicability;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stored_facts::types::{ArenaBytes, Bucket, FileNotes, LanguageNumber};
use ryo_values::domain::stores::types::{BucketsWanted, ReadHanded, Stored};

/// A file, by its place among the files in the order of their paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileOrder(pub usize);

/// How many files there are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileTotal(pub usize);

/// How many files a worker read, not counting those the store kept.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FilesRead(pub u32);

/// A file read now and its record, as a worker keeps them until the workers meet.
#[derive(Clone, Debug)]
pub(crate) struct FreshRecord {
    /// The file's place among the files.
    pub place: FileOrder,
    /// What came of the file.
    pub record: Record,
}

/// What the language wrote of one file, for the project to read, and the file's place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlacedNotes {
    /// The file's place among the files.
    pub place: FileOrder,
    /// The language that wrote the notes, and the notes.
    pub notes: FileNotes,
}

/// What the workers wrote of the files they read, to be put in the order of the files.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FreshNotes(pub Vec<PlacedNotes>);

/// What a run uses again of a file the store kept, when it does not read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Replaying {
    /// Its findings: the steps that read across files are not made again.
    Findings,
    /// Its findings, and what it hands the steps that read across files and the project, which are made again.
    FindingsAndHanded,
}

/// Whether a visit reads the memory the process holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Checking {
    /// This visit reads it.
    Now,
    /// This one does not.
    Later,
}

/// What one worker owns, which no other worker touches while the jobs run: its element table, the bytes of the file it reads,
/// the rules that govern it, its tables of the store and the room it computes in.
pub(crate) struct Tables {
    /// Current file's demanded named frontier readiness; empty ordinary path allocates nothing.
    pub current_observations:
        Option<std::sync::Arc<ryo_values::domain::records::types::FrontierReadiness>>,
    /// Actual fresh/replayed readiness accumulated for the existing cross handover.
    pub observations:
        Vec<ryo_execution::inbound::use_cases::metadata::interface::FileObservationReadiness>,
    /// The worker's number.
    pub number: WorkerNumber,
    /// The elements of the file being read.
    pub elements: Elements,
    /// The names of the file being read.
    pub symbols: Symbols,
    /// The bytes of the file being read.
    pub bytes: FileBytes,
    /// The language that read the file being read, when one did.
    pub language: Option<LanguageNumber>,
    /// The rules that govern the file.
    pub mask: Mask,
    /// The globs that matched the file.
    pub hits: Hits,
    /// The words of the table being handed over.
    pub words: Words,
    /// The words of the table being handed over, packed as they are kept.
    pub packed: PackedWords,
    /// Actual work totals returned by the execution owner.
    pub fed: Vec<Vec<RowsFed>>,
    /// The active immutable vocabulary; no operator/cache owner is held here.
    pub kind_vocabulary: Option<std::sync::Arc<PreparedVocabulary>>,
    /// Where the lines of the file start.
    pub starts: LineStarts,
    /// The text the names of the files made.
    pub arena: ArenaBytes,
    /// Where the text of the names of the table being handed over is.
    pub places: Vec<NamePlace>,
    /// The findings of the file being read.
    pub rendered: Rendered,
    /// The files this worker could not read.
    pub unread: Vec<Unread>,
    /// The files this worker read whose findings cannot be kept for another run: the next run reads them again.
    pub unrecorded: Vec<FilePath>,
    /// What the file being read handed to the cross-file steps, bucket by bucket.
    pub captured: Vec<Kept>,
    /// What this worker made of the files it read, to be kept: the place of the file among the files, and the record.
    pub fresh: Vec<FreshRecord>,
    /// What the language wrote of each file this worker read, by the file's place, for the project.
    pub notes: Vec<PlacedNotes>,
    /// How many files this worker read, not counting those the store kept.
    pub reads: FilesRead,
    /// The relations the file being visited is read in part, each with its cause.
    pub partials: Vec<Partial>,
    /// The files this worker visited that have a relation read in part, which a rule's cross-file steps trust.
    pub covered: Vec<Covered>,
    /// What the rules could not decide in the files this worker visited.
    pub unknowns: Vec<Unknown>,
    /// Rules explicitly skipped for files whose syntax they require was rejected.
    pub skipped: Vec<Skipped>,
}

/// What a visit of a file reads that is the same for every file.
#[derive(Clone)]
pub(crate) struct VisitSetting<'a> {
    /// Pure source and accounting facts extracted once before executable plans are consumed.
    pub metadata: &'a [ryo_values::domain::run_metadata::types::RuleRunMetadata],
    /// The relations the languages are asked to walk the files for.
    pub wanted: &'a [Relation],
    /// The path of each file of the run.
    pub paths: &'a [FilePath],
    /// The first bucket of each rule's frontier.
    pub first_bucket: &'a [Bucket],
    /// The ids of the rules.
    pub ids: &'a [RuleId],
    /// Whether each rule runs on every file or only after syntax acceptance.
    pub applicability: &'a [RuleApplicability],
    /// Whether any rule can be gated or its applicability metadata is incomplete.
    pub applicability_can_gate: bool,
    /// Which rule governs which file.
    pub placed: &'a PlacedScopes,
    /// What the store kept of each file, by the file's place among the files.
    pub records: &'a [Option<&'a Record>],
    /// What the run uses again of a file the store kept and the run does not read.
    pub replaying: Replaying,
    /// What the last run kept, which what a file it does not read handed is taken from.
    pub stored: &'a Stored,
    /// What the files of the last run handed, read when the run makes the cross-file steps again.
    pub handed: &'a ReadHanded,
    /// The buckets the cross-file steps made again read.
    pub wanted_buckets: &'a BucketsWanted,
    /// The most memory the process may hold while it runs.
    pub budget: MemoryBytes,
    /// Whether the languages write their notes of the files.
    pub noted: Noting,
    /// Whether the languages give the elements their roles, for the queries that read them.
    pub roled: RoleRead,
    /// The relations the steps of each rule trust the absence of rows of.
    pub needs: &'a [Needs],
    /// How the rows the rules emit are answered.
    pub answering: &'a Answering,
    /// Whether the run's store keeps anything.
    pub keeping: Keeping,
    /// What the reasons of the languages linked are called.
    pub unknown: &'a [UnknownName],
}

/// What the workers made, once they have met: their text, the files they could not read and what the rules could not decide.
#[derive(Debug, Default)]
pub(crate) struct WorkersMet {
    pub observations:
        Vec<ryo_execution::inbound::use_cases::metadata::interface::FileObservationReadiness>,
    /// The text each worker made, in the order of the workers.
    pub arenas: Vec<ArenaBytes>,
    /// The files no language read.
    pub unread: Vec<Unread>,
    /// The files the workers read whose findings cannot be kept for another run.
    pub unrecorded: Vec<FilePath>,
    /// The files that have a relation read in part.
    pub covered: Vec<Covered>,
    /// What the rules could not decide.
    pub unknowns: Vec<Unknown>,
    /// Rules explicitly skipped for files whose syntax they require was rejected.
    pub skipped: Vec<Skipped>,
    /// What the workers made of the files they read, in no order.
    pub fresh: Vec<FreshRecord>,
    /// What the languages wrote of the files read now.
    pub notes: FreshNotes,
    /// How many files were read, not counting those the store kept.
    pub read: FilesRead,
    /// How many rows each step of each rule was fed: by the workers over the files, then by the cross-file finish.
    pub fed: Vec<Vec<RowsFed>>,
}

/// The records this run made, with each one's place among the files, in the order of the files.
#[derive(Clone, Debug, Default)]
pub(crate) struct Fresh {
    /// The place of each record.
    pub places: Vec<FileOrder>,
    /// The records.
    pub records: Vec<Record>,
}

/// What the workers leave when they have read every file: the store's handles of each, what they met as, and how many findings
/// they streamed.
pub(crate) struct Visited<W> {
    /// The store's handle of each worker, of the type the store hands out.
    pub workers: Vec<W>,
    /// What the workers met as.
    pub met: WorkersMet,
    /// How many findings they streamed.
    pub counted: RowTotal,
    /// How many rows the files had, of which `counted` were streamed: more when the answer has a limit.
    pub total: RowTotal,
}

/// What the project wrote, in the run's own handle: its names and its rows.
pub(crate) struct ProjectHandle<W> {
    /// The names the project wrote.
    pub symbols: Symbols,
    /// The rows the project wrote.
    pub facts: W,
}

/// What the rules that cross files read when they are made again: what the files handed over and the text of the workers.
pub(crate) struct CrossReading<'a, W> {
    /// Actual fresh/replayed per-file named-root readiness.
    pub observations:
        &'a [ryo_execution::inbound::use_cases::metadata::interface::FileObservationReadiness],
    /// The text of the workers.
    pub arenas: &'a mut Vec<ArenaBytes>,
    /// What each rule covered.
    pub covered: &'a [Covered],
    /// Files explicitly skipped because syntax was rejected.
    pub skipped: &'a [Skipped],
    /// The store's handle of each worker.
    pub workers: Vec<W>,
}
