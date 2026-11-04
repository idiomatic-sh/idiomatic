//! Linter integrations bind commands to decoding; execution stays shared.
use crate::domain::diagnostics::{errors::DiagnosticsError as Error, types::RunReport};
use crate::domain::measurements::types::{Invocation, ProcessOutput};
use crate::domain::suite::types::RunRequest;

/// Bind one linter invocation to its canonical result decoding.
pub trait LinterPort {
    /// Whether this concrete adapter honors the shared explicit rule configuration contract.
    fn rule_configuration(&self) -> crate::domain::suite::types::RuleConfigurationSupport;
    /// Construct an invocation over the exact supplied input set.
    ///
    /// # Errors
    ///
    /// Returns an error when the request cannot be represented by this linter.
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error>;
    /// Normalize a completed process result outside the measured execution interval.
    ///
    /// # Errors
    ///
    /// Returns an error for operational failure, malformed output or inconsistent coverage.
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error>;
}
