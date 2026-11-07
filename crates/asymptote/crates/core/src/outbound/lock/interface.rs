//! What the `lock` port asks of the outside, in the core's own vocabulary.

use super::errors::LockError;
use super::types::{Listing, Operations};
use crate::domain::manifest::types::Pathspec;
use crate::domain::operations::types::{Operation, OperationId, OperationText};
use crate::domain::violations::types::{Id, Key};

/// What every `lock` adapter implements, over the lock of the product it is bound to.
pub trait LockPort: std::fmt::Debug + Send + Sync {
    /// The lock's operation files; none when the product has no lock yet.
    ///
    /// # Errors
    ///
    /// `Unreadable` when its directory cannot be listed.
    fn listing(&self) -> Result<Listing, LockError>;

    /// Every operation of the lock, each verified against the content address its file is named by.
    ///
    /// # Errors
    ///
    /// `Unreadable`, `Malformed` or `Address` when one cannot be read, parsed or verified.
    fn read(&self) -> Result<Operations, LockError>;

    /// One operation as another revision holds it, verified against `id`.
    ///
    /// # Errors
    ///
    /// `Malformed` or `Address` when it cannot be parsed or verified.
    fn parse(&self, id: &OperationId, text: &OperationText) -> Result<Operation, LockError>;

    /// Appends `operation` under its content address. An operation is immutable: the same text again is no change.
    ///
    /// # Errors
    ///
    /// `Unwritable` when it cannot be written, `Address` when other text holds its address.
    fn append(&self, operation: &Operation) -> Result<(), LockError>;

    /// Writes `snapshot` under its content address, then deletes the file of every operation it covers, so the lock
    /// holds the snapshot and what was appended beside it. A covered file already gone is no failure.
    ///
    /// # Errors
    ///
    /// `Unwritable` when the snapshot cannot be written or a covered file cannot be deleted, `Address` when other
    /// text holds the snapshot's address.
    fn compact(&self, snapshot: &Operation) -> Result<(), LockError>;

    /// Where the lock keeps its operations, relative to the product: what an earlier revision's lock is read from.
    fn directory(&self) -> Pathspec;

    /// The content address of `key`, the id its records carry.
    fn id(&self, key: &Key) -> Id;
}
