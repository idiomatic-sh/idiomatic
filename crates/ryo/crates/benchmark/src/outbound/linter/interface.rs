//! Linter integrations bind commands to decoding; execution stays shared.
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;

/// Bind one linter invocation to its canonical result decoding.
pub trait LinterPort {
    /// Whether this concrete adapter honors the shared explicit rule configuration contract.
    fn rule_configuration(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport;
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
