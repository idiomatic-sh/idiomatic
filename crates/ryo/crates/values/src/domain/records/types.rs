//! The types of the `records` concept: what a run found of each file. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::files::types::FileNumber;
use crate::domain::files::types::FileSize;
use crate::domain::files::types::Nanoseconds;
use crate::domain::row_texts::types::{FindingRows, PlacedRule};
use crate::domain::stored_facts::types::Bucket;
use crate::domain::stored_facts::types::Digest;
use crate::domain::stored_facts::types::ReadFailure;
use crate::domain::stored_facts::types::{ArenaBytes, FileNotes, StoredPartial};
use crate::domain::stored_facts::types::{KeptWords, PackedWords};
use ryo_language::domain::files::types::FilePath;

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

/// Whether a file handed the readiness of its demanded named frontiers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum HandedReadiness {
    /// The readiness the file recorded.
    Recorded(std::sync::Arc<FrontierReadiness>),
    /// Missing or ambiguous: unavailable to demanded roots.
    #[default]
    Unrecorded,
}

/// What a file hands the steps that read across files and the project: its words in each bucket, with the numbers that differ from
/// run to run taken out, the text its names made that is in no file, what its language noted of it, and the relations it is read
/// in part with the rules that govern it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Handed {
    /// Independently versioned readiness of demanded per-file named frontiers.
    pub observations: HandedReadiness,
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
    pub offset: PayloadOffset,
    /// Exact packed payload length, with no framing bytes.
    pub length: PayloadLength,
}

/// A byte position of packed words in a spool or a snapshot file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PayloadOffset(pub u64);

/// The exact byte length of packed words in a spool or a snapshot file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PayloadLength(pub u64);

/// The byte position where a stored row's header starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HeaderOffset(pub u64);

/// A fresh row's position among both inline and staged rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StagedOrdinal(pub u64);

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

/// A producer rule's ordinal in the request that wrote a bucket.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProducerRule(pub u32);

/// A frontier's original ordinal within its producer rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrontierOrdinal(pub u32);

/// Persisted root identity. The containing file and bucket remain part of the
/// enclosing record; a new owner validates this proof against its current rule
/// digest, frontier ordinal, physical file and observed coverage before use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontierProof {
    /// Producer rule ordinal in the request that wrote this bucket.
    pub rule: ProducerRule,
    /// Original frontier ordinal within that rule.
    pub input: FrontierOrdinal,
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

/// Whether a file is as the store kept it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freshness {
    /// The system says nothing of it changed.
    AsItWas,
    /// It changed, or the system cannot tell.
    Changed,
}

/// Compact actual readiness beside existing frontier words. Slots are bound to the stable semantic layout identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontierReadiness {
    /// Encoding version; old versions cannot establish completion.
    pub version: crate::domain::records::types::ObservationReadinessVersion,
    /// Stable ordered producer/output/query/config identity for these bits.
    pub layout: crate::domain::records::types::ObservationReadinessLayout,
    /// Exact number of logical roots represented.
    pub slots: crate::domain::records::types::ObservationReadinessCount,
    /// Successful production bits, including actual complete zero-row outputs.
    pub complete: ReadinessBits,
    /// Explicit not-applicable bits; empty storage means no skips, never completion.
    pub skipped: ReadinessBits,
    /// Only explicit failures need owned detail. An unset slot is Unknown, never Complete.
    pub refusals: Vec<FrontierRefusal>,
}
/// Slot bits of one readiness map, 64 slots to a word.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReadinessBits(pub Vec<u64>);

#[derive(Clone, Debug, PartialEq, Eq)]
/// One explicitly unavailable root; ordinary pending slots need no owned reason.
pub struct FrontierRefusal {
    /// Position in the enclosing validated map.
    pub slot: crate::domain::records::types::ObservationReadinessSlot,
    /// Why production cannot establish complete rows.
    pub reason: FrontierRefusalReason,
}
#[derive(Clone, Debug, PartialEq, Eq)]
/// A typed unavailable state, never an empty successful population.
pub enum FrontierRefusalReason {
    /// A real provider, dependency or retention refusal.
    Unknown(ObservationReason),
    /// The selected producer has no executable query.
    NoQuery,
}

/// Whether a published root carries the words of its table.
#[derive(Clone, Debug)]
pub enum RootWords {
    /// The packed words of the root's table.
    Published(crate::domain::stored_facts::types::KeptWords),
    /// The root carries no words: its readiness says why.
    Withheld,
}

/// A portable named output. Complete words, or partial words whose readiness
/// remains Unknown, require a matching engine-issued publication at admission.
#[derive(Clone, Debug)]
pub struct PackedObservedRoot {
    pub output: ObservationIndex,
    pub words: RootWords,
    pub readiness: crate::domain::records::types::ProducerReadiness,
    /// Every arena actually referenced by encoded places, including inherited re-exports.
    pub arenas: Vec<ryo_relations::domain::relations::types::WorkerNumber>,
}
/// One closure owns its reserved arena and returns only owned bytes/words.
#[derive(Clone, Debug)]
pub struct CrossPublicationTransfer {
    pub worker: ryo_relations::domain::relations::types::WorkerNumber,
    pub arena: ArenaBytes,
    pub roots: Vec<PackedObservedRoot>,
}

/// Version of persisted root-readiness evidence, independent of row content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservationReadinessVersion(pub u32);

/// Semantic identity of ordered producer/output definitions plus query/config/schema identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservationReadinessLayout(pub crate::domain::stored_facts::types::Digest);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Number of logical status slots in the current semantic layout.
pub struct ObservationReadinessCount(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// Position within one validated semantic layout; not an independently stable identity.
pub struct ObservationReadinessSlot(pub u32);

/// Why a declared producer cannot supply an observed result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationReason(pub Box<str>);

/// A declared output's local position within one producer plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationIndex(pub u32);

/// Readiness belongs to an actually admitted output, distinct from selected-rule inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProducerReadiness {
    /// The producer root completed, including a real zero-row result.
    Complete,
    /// A dependency, mapping or retention refusal prevents a complete result.
    Unknown(ObservationReason),
    /// The producer is not applicable in this file/scope.
    Skipped,
    /// The configured producer has no executable query; it is never treated as complete-empty.
    NoQuery,
}
