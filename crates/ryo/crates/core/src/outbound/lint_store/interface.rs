//! What the `lint_store` port asks of the outside, in the core's own vocabulary: the tables a lint keeps for the next run. Where they are
//! and which build made them are the adapter's settings, never asked here.

use std::sync::Arc;

use super::errors::LintStoreError;
use super::types::StoreKeeping;
use ryo_execution::inbound::use_cases::prepare::interface::CachedPlan;
use ryo_values::domain::stores::types::{BucketsWanted, KeptRows, ReadHanded, Stored};

/// A worker-owned stream of old rows. Postcard may observe a row when its
/// bounded bucket buffer fills, before the caller asks for that row. The
/// independent `ReadStore::read_old` contract remains an exact per-call read.
pub trait OldRowBatch: Send {
    /// Resolve the next old descriptor in this worker's increasing file order.
    fn read(
        &mut self,
        words: &ryo_values::domain::stores::types::OldWords,
        into: &mut ryo_values::domain::stored_facts::types::PackedWords,
    ) -> Result<(), LintStoreError>;
}

/// What every `lint_store` adapter implements.
pub trait LintStorePort: Send + Sync {
    /// Whether this keeps anything: a run over one that does not makes nothing only a store reads.
    fn keeps(&self) -> StoreKeeping;

    /// The store as the last successful run of this build left it, read for one run: it holds nothing when nothing is kept, or what
    /// is kept is another build's, or does not read back as the tables. What the run asks of it next, and what it keeps, are of the
    /// store it read, whatever another run keeps meanwhile.
    ///
    /// # Errors
    ///
    /// `Unreadable` when the place cannot be read.
    fn stored(&self) -> Result<Arc<dyn ReadStore>, LintStoreError>;

    /// The compiled plans the last run of this build kept, each with the compilation identity it was compiled under: none when
    /// nothing is kept, the plans are another build's, or they do not read back. A plan is used only by a rule whose identity it
    /// still matches, so the plans are a cache: one missing or unreadable is compiled again, and no answer changes.
    fn plans(&self) -> Vec<CachedPlan>;

    /// Keeps `plans` in place of the plans kept, for the next run of this build.
    ///
    /// # Errors
    ///
    /// `Unwritable` when the place cannot be written.
    fn keep_plans(&self, plans: &[CachedPlan]) -> Result<(), LintStoreError>;
}

/// The store exactly as one run read it.
pub trait ReadStore: Send + Sync + std::fmt::Debug {
    /// What the last run kept but what its files handed: empty when the store holds nothing.
    fn tables(&self) -> Stored;

    /// What the files handed the steps that read across files and the project when the last run kept its store, left out of
    /// `tables`, with the words of the buckets `wanted` alone: empty when `tables` is.
    ///
    /// # Errors
    ///
    /// `Unreadable` when the place cannot be read.
    fn handed(&self, wanted: &BucketsWanted) -> Result<ReadHanded, LintStoreError>;

    /// Resolve one old row from this exact snapshot into reusable caller-owned storage.
    ///
    /// # Errors
    ///
    /// Foreign identities, overflow, invalid row framing and requested read failures are refused.
    fn read_old(
        &self,
        _words: &ryo_values::domain::stores::types::OldWords,
        _into: &mut ryo_values::domain::stored_facts::types::PackedWords,
    ) -> Result<(), LintStoreError>;

    /// Open a worker-owned bounded stream over the already-open snapshot.
    /// `None` retains the direct per-row path for adapters without batching.
    fn begin_old_rows(
        &self,
        _wanted: &BucketsWanted,
    ) -> Result<Option<Box<dyn OldRowBatch>>, LintStoreError>;

    /// Retain successfully normalized fresh words until this run keeps or drops its snapshot.
    /// Custom in-memory stores may keep inline representation; no IO is required.
    ///
    /// # Errors
    ///
    /// Staging creation/write failures are propagated, never replaced with an empty row.
    fn stage_words(
        &self,
        words: ryo_values::domain::stored_facts::types::PackedWords,
    ) -> Result<ryo_values::domain::records::types::RecordWords, LintStoreError>;

    /// Resolve only this snapshot's descriptor into reusable caller-owned storage.
    ///
    /// # Errors
    ///
    /// Foreign/stale descriptors and span/read/flush failures are refused.
    fn read_staged(
        &self,
        _words: &ryo_values::domain::records::types::StagedWords,
        _into: &mut ryo_values::domain::stored_facts::types::PackedWords,
    ) -> Result<(), LintStoreError>;

    /// Keeps `stored` in place of what was kept: its words are those of the files the run read; the words of the buckets
    /// `kept.unchanged` are kept as they were, and in every other bucket the words of each file of `kept.file_again` are kept as they
    /// were, under its number now. The numbers of `kept` are those of the store this read, and the words are copied from that store
    /// whatever another run kept since. A reader sees the old tables or the new, never part of them.
    ///
    /// # Errors
    ///
    /// `Unwritable` when the place cannot be written, or `kept` names words of a store that holds none.
    fn keep(&self, stored: Stored, kept: &KeptRows) -> Result<(), LintStoreError>;
}
