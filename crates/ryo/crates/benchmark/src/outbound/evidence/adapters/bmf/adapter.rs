//! Atomic document encoding precedes writing to the injected destination.
use super::schemas;
use crate::domain::{diagnostics::errors::Error, measurements::types::Sample};
use crate::outbound::evidence::interface::EvidencePort;
use std::{io::Write, sync::Mutex};

pub(crate) struct EvidenceBmfAdapter {
    output: Mutex<Box<dyn Write + Send>>,
}
impl EvidenceBmfAdapter {
    pub(crate) fn new(output: Box<dyn Write + Send>) -> Self {
        Self {
            output: Mutex::new(output),
        }
    }
}
impl std::fmt::Debug for EvidenceBmfAdapter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
impl EvidencePort for EvidenceBmfAdapter {
    fn publish(&self, samples: &[Sample]) -> Result<(), Error> {
        let bytes = schemas::encode(samples)?;
        let mut output = self
            .output
            .lock()
            .map_err(|_| Error::Invalid("evidence output lock poisoned".into()))?;
        output.write_all(&bytes)?;
        output.write_all(b"\n")?;
        output.flush()?;
        Ok(())
    }
}
