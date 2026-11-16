//! An injected destination stream owned by the evidence adapter.
use super::types::MeasureDocument;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::{io::Write, sync::Mutex};

/// Resource lifetime is bounded by the configured adapter instance.
pub(crate) struct EvidenceBmfConnection {
    /// Serialized access to the externally supplied evidence destination.
    pub output: Mutex<Box<dyn Write + Send>>,
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
    /// The document goes to a new file at `document`, and the comparison evidence to `comparison`.
    ///
    /// # Errors
    /// Refuses a document file that cannot be created.
    pub(crate) fn document(
        document: &std::path::Path,
        comparison: Box<dyn Write + Send>,
    ) -> Result<Self, Error> {
        Ok(Self {
            output: Mutex::new(Box::new(std::fs::File::create(document)?)),
            comparison: Mutex::new(comparison),
            document: Mutex::new(Some(MeasureDocument::default())),
        })
    }
    /// The document goes to the file at `destination`, appended to, or nowhere when the run names none; the comparison
    /// evidence goes nowhere.
    ///
    /// # Errors
    /// Refuses a destination that cannot be opened for appending.
    pub(crate) fn appending(destination: Option<&std::path::Path>) -> Result<Self, Error> {
        let output: Box<dyn Write + Send> = match destination {
            Some(path) => Box::new(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)?,
            ),
            None => Box::new(std::io::sink()),
        };
        Ok(Self {
            output: Mutex::new(output),
            comparison: Mutex::new(Box::new(std::io::sink())),
            document: Mutex::new(Some(MeasureDocument::default())),
        })
    }
}
