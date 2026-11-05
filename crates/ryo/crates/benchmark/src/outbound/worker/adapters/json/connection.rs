//! The externally supplied output stream and its synchronization lifetime.
use super::constants::{OUTPUT_LOCK_POISONED, RESPONSE_TERMINATOR};
use super::types::WorkerResponseBytes;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::{io::Write, sync::Mutex};

/// Owns the stream opened by the process composition; it never buffers reports across calls.
pub(crate) struct WorkerJsonConnection {
    /// Serialized access to the externally supplied response stream.
    output: Mutex<Box<dyn Write + Send>>,
}
impl WorkerJsonConnection {
    /// Take ownership of the response stream.
    pub(crate) fn new(output: Box<dyn Write + Send>) -> Self {
        Self {
            output: Mutex::new(output),
        }
    }
    /// Write one complete JSON response, terminate it, and flush the stream.
    pub(crate) fn publish(&self, bytes: &WorkerResponseBytes) -> Result<(), Error> {
        let mut output = self
            .output
            .lock()
            .map_err(|_poisoned| Error::Invalid(OUTPUT_LOCK_POISONED.into()))?;
        output.write_all(&bytes.0)?;
        output.write_all(RESPONSE_TERMINATOR)?;
        output.flush()?;
        Ok(())
    }
}
impl std::fmt::Debug for WorkerJsonConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
