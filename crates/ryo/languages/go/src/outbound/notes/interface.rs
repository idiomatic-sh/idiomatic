//! What the `notes` port asks of the outside: the bytes the parse leaves of a file for the project to read, written from the language's
//! own types.

use super::errors::NotesError;
use crate::domain::fact_notes::types::Note;
use ryo::languages::Bytes;

/// What every `notes` adapter implements.
pub(crate) trait NotesPort: Send + Sync {
    /// Writes `note` into `out`, in place of what it held.
    fn write(&self, note: &Note, out: &mut Bytes);

    /// The note `bytes` hold.
    ///
    /// # Errors
    ///
    /// `NotANote` when they do not hold exactly one: bytes cut short, or followed by more, are none.
    fn note_in(&self, bytes: &Bytes) -> Result<Note, NotesError>;
}
