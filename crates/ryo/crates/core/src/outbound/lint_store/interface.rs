//! What the `lint_store` port asks of the outside, in the core's own vocabulary: the tables a lint keeps for the next run. Where they are
//! and which build made them are the adapter's settings, never asked here.

use std::sync::Arc;

use super::errors::LintStoreError;
use super::types::StoreKeeping;
use ryo_values::domain::stores::types::{BucketsWanted, KeptRows, Stored, StoredHanded};

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
    fn handed(&self, wanted: &BucketsWanted) -> Result<StoredHanded, LintStoreError>;

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
