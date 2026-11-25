//! An injected destination stream owned by the evidence adapter.
use super::types::{EncodedDocument, MeasureDocument};
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::{io::Write, sync::Mutex};

/// Where the one document is written when the run finishes; a file is opened only then.
pub(crate) enum DocumentDestination {
    /// A stream the composition supplied.
    Stream(Mutex<Box<dyn Write + Send>>),
    /// A new file at this path.
    Created(std::path::PathBuf),
    /// The file at this path, appended to.
    Appended(std::path::PathBuf),
}

/// Resource lifetime is bounded by the configured adapter instance.
pub(crate) struct EvidenceBmfConnection {
    /// Where the document goes.
    pub output: DocumentDestination,
    /// Serialized access to the externally supplied comparison evidence destination.
    pub comparison: Mutex<Box<dyn Write + Send>>,
    /// Every measure added so far, written once by `finish`; `None` once it is written.
    pub document: Mutex<Option<MeasureDocument>>,
}
impl std::fmt::Debug for EvidenceBmfConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
impl EvidenceBmfConnection {
    /// Write the whole document to its destination, opening a file destination now.
    ///
    /// # Errors
    /// Refuses a destination that cannot be opened or written.
    pub(crate) fn write_document(&self, document: &EncodedDocument) -> Result<(), Error> {
        let bytes = document.0.as_slice();
        let mut file = match &self.output {
            DocumentDestination::Stream(stream) => {
                let mut output = stream
                    .lock()
                    .map_err(|_poisoned| Error::Invalid(super::constants::POISONED.into()))?;
                output.write_all(bytes)?;
                output.flush()?;
                return Ok(());
            }
            DocumentDestination::Created(path) => std::fs::File::create(path)?,
            DocumentDestination::Appended(path) => std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?,
        };
        file.write_all(bytes)?;
        file.flush()?;
        Ok(())
    }
}
