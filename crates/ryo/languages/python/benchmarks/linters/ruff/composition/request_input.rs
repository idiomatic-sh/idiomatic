//! Parse the child's request transport without putting large JSON in argv.
use super::targets::TargetConfiguration;
use crate::constants;
use crate::constants::WORKER_REQUIRES_AN_EXPLICIT_SUPPOR;
use ryo_benchmark::inbound::use_cases::check_worker_request::types::WorkerExpectation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::ParserConfigurationName;
use std::path::PathBuf;

/// Text supplied as one executable command argument.
pub(super) struct BenchmarkArgument(pub String);

/// Where one worker request is read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum WorkerInput {
    /// A request file the parent owns.
    File(PathBuf),
    /// The inline JSON debug argument.
    Inline(InlineRequest),
}

/// The JSON text of one inline worker request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct InlineRequest(pub(super) String);

/// Name an explicit request file or the inline JSON debug argument.
pub(super) fn worker_input(
    args: &mut impl Iterator<Item = BenchmarkArgument>,
) -> Result<WorkerInput, Error> {
    let input_argument = args
        .next()
        .map(|argument| argument.0)
        .ok_or_else(|| Error::Invalid(constants::MISSING_WORKER_REQUEST.into()))?;
    if input_argument == constants::REQUEST_FILE {
        args.next()
            .map(|argument| argument.0)
            .map(PathBuf::from)
            .map(WorkerInput::File)
            .ok_or_else(|| Error::Invalid(constants::MISSING_WORKER_REQUEST.into()))
    } else {
        Ok(WorkerInput::Inline(InlineRequest(input_argument)))
    }
}

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
        .ok_or_else(|| Error::Invalid(WORKER_REQUIRES_AN_EXPLICIT_SUPPOR.into()))?;
    Ok(WorkerExpectation {
        parser_configuration: ParserConfigurationName(configuration.0.into()),
        pinned_applicability: crate::schemas::pinned_rule_phases()?,
    })
}
