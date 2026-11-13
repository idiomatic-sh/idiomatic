//! The identity the worker's parser and pinned phases require of every request it reads.
use super::targets::TargetConfiguration;
use crate::constants::WORKER_REQUIRES_A_SUPPORTED_PARSER;
use ryo_benchmark::inbound::use_cases::check_worker_request::types::WorkerExpectation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::ParserConfigurationName;

/// The identity the executable's parser and pinned phases require of every request it runs.
///
/// # Errors
///
/// Missing or unsupported target identities are not executions of a canonical staged profile.
pub(super) fn worker_expectation(
    configuration: Option<TargetConfiguration<'_>>,
) -> Result<WorkerExpectation, Error> {
    let configuration = configuration
        .filter(|value| super::targets::resolve(Some(*value)).is_ok())
        .ok_or_else(|| Error::Invalid(WORKER_REQUIRES_A_SUPPORTED_PARSER.into()))?;
    Ok(WorkerExpectation {
        parser_configuration: ParserConfigurationName(configuration.0.into()),
        pinned_applicability: crate::schemas::pinned_rule_phases()?,
    })
}
