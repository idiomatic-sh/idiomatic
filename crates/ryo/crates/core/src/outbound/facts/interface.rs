//! What the `facts` port asks of the outside: for each worker a handle of its own to put the rows of the file it is reading and the
//! words it keeps of it, and the cross-file finish where the workers' words meet. Nothing here locks: a handle belongs to one worker.

use ryo_language::domain::relations::types::Relation;

use super::errors::FactsError;
use super::types::{ColumnsMut, WordsToKeep};
use ryo_language::domain::relations::types::ColumnsView;
use ryo_values::domain::stored_facts::types::{Bucket, KeptWords};

/// What every `facts` adapter implements.
pub trait FactsPort: Send + Sync {
    /// Opens the handle one worker works through, with room for every relation.
    ///
    /// # Errors
    ///
    /// `Store` when the adapter cannot make it.
    fn worker(&self) -> Result<Box<dyn FactsWorker>, FactsError>;

    /// The cross-file finish: brings every worker's words together, one worker after another, and takes the handles with it.
    ///
    /// # Errors
    ///
    /// `Store` when the words cannot be brought together; `File` when a file fails.
    fn finish(
        &self,
        workers: &mut dyn Iterator<Item = Box<dyn FactsWorker>>,
    ) -> Result<Box<dyn FactsFinish>, FactsError>;
}

/// One worker's handle: where its rows and its kept words are, which no other worker reads or writes.
pub trait FactsWorker: Send + Sync {
    /// The columns of every relation that the worker's file writes to, asked for once for the file. A language's rows are pushed onto
    /// them, once, and read from there: a port is called once for a file, never for a row.
    ///
    /// # Errors
    ///
    /// `Store` when the handle cannot give them.
    fn columns_mut(&mut self) -> Result<ColumnsMut<'_>, FactsError>;

    /// The rows of a relation that the worker's file wrote, borrowed in place from resident columns.
    /// This lookup performs no I/O; loading and persistence belong to the other operations.
    ///
    /// # Errors
    ///
    /// `Store` when the handle cannot read them.
    fn rows(&self, relation: Relation) -> Result<ColumnsView<'_>, FactsError>;

    /// Adds words to a bucket: what the file leaves for the finish.
    ///
    /// # Errors
    ///
    /// `Store` when the handle refuses them; `File` when a file fails.
    fn keep(&mut self, bucket: Bucket, words: WordsToKeep<'_>) -> Result<(), FactsError>;

    /// Empties the rows of the worker's file, keeping the room for the next one. The buckets keep what was added to them.
    ///
    /// # Errors
    ///
    /// `Store` when the handle cannot.
    fn release(&mut self) -> Result<(), FactsError>;

    /// What the worker kept, finished: the handle is taken, and its words are read from what it gives.
    ///
    /// # Errors
    ///
    /// `Store` when the words cannot be finished; `File` when a file fails.
    fn finished(self: Box<Self>) -> Result<Box<dyn FactsFinish>, FactsError>;
}

/// What the finish made of the workers' words.
pub trait FactsFinish: Send + Sync {
    /// Everything the workers added to a bucket, one worker after another; nothing for a bucket nobody added to.
    ///
    /// # Errors
    ///
    /// `Store` when it cannot be read; `File` when a file fails.
    fn kept(&self, bucket: Bucket) -> Result<KeptWords, FactsError>;
}
