//! The types of the `persisting` operation: what a run that keeps its answer reads of itself.
use crate::domain::render::types::Answering;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::{BucketsWanted, ReadHanded, Reuse};

/// What a run reads of itself when it keeps what it found for the next.
pub(crate) struct KeepRun<'a> {
    /// How the rows the rules emit are answered: only findings are kept.
    pub answering: &'a Answering,
    /// What the run used again of what the last kept.
    pub again: &'a Reuse,
    /// The files whose stored rows are theirs no more.
    pub read_again: &'a [FilePath],
    /// What the last run of this build kept.
    pub stored: &'a ryo_values::domain::stores::types::Stored,
    /// Pure rule facts retained for source demand, frontier and work accounting.
    pub metadata: &'a [ryo_values::domain::run_metadata::types::RuleRunMetadata],
    /// The first bucket of each rule's frontier.
    pub first_bucket: &'a [Bucket],
    /// The buckets of what the files handed that the run read.
    pub wanted: &'a BucketsWanted,
    /// What the files handed the last run, as far as the run read it.
    pub handed: &'a ReadHanded,
}
