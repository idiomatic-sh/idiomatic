//! Deterministic SDK substitute preserves the submitted file set.
use crate::domain::diagnostics::{errors::Error, types::RunReport};
use crate::domain::suite::types::WorkerRequest;
use crate::outbound::engine::interface::EnginePort;
#[derive(Debug, Default)]
pub struct EngineFake {
    pub refuses: bool,
}
impl EnginePort for EngineFake {
    fn run(&self, request: WorkerRequest) -> Result<RunReport, Error> {
        if self.refuses {
            return Err(Error::Invalid("no candidate queries".into()));
        }
        Ok(RunReport {
            files: request.files,
            diagnostics: Vec::new(),
        })
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
