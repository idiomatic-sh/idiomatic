//! Parse the child's request transport without putting large JSON in argv.
use super::constants;
use super::constants::{
    ACCEPTED_SYNTAX, ALWAYS, APPLICABILITY, PARSER_CONFIGURATION, RULE_APPLICABILITY,
    RULES_DIRECTORY, WORKER_PARSER_CONFIGURATION_DIFFER, WORKER_REQUIRES_AN_EXPLICIT_SUPPOR,
    WORKER_REQUIRES_EXPLICIT_SELECTED, WORKER_RULE_APPLICABILITY_DIFFERS,
};
use super::targets::TargetConfiguration;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::path::PathBuf;

/// Text supplied as one executable command argument.
pub(super) struct BenchmarkArgument(pub String);

/// The JSON text of one worker request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WorkerInput(pub(super) String);

/// Read an explicit request file or the inline JSON debug argument.
pub(super) fn worker_input(
    args: &mut impl Iterator<Item = BenchmarkArgument>,
) -> Result<WorkerInput, Error> {
    let input_argument = args
        .next()
        .map(|argument| argument.0)
        .ok_or_else(|| Error::Invalid(constants::MISSING_WORKER_REQUEST.into()))?;
    if input_argument == constants::REQUEST_FILE {
        let path = args
            .next()
            .map(|argument| argument.0)
            .map(PathBuf::from)
            .ok_or_else(|| Error::Invalid(constants::MISSING_WORKER_REQUEST.into()))?;
        std::fs::read_to_string(path)
            .map(WorkerInput)
            .map_err(Into::into)
    } else {
        Ok(WorkerInput(input_argument))
    }
}

/// Bind the executable's parser and phase choices to the actual explicit request.
///
/// # Errors
///
/// Missing, unsupported or conflicting target/phase identities are not executions
/// of a canonical staged profile.
pub(super) fn checked_worker_identity(
    input: &WorkerInput,
    configuration: Option<TargetConfiguration<'_>>,
) -> Result<(), Error> {
    let configuration = configuration
        .filter(|value| super::targets::resolve(Some(*value)).is_ok())
        .ok_or_else(|| Error::Invalid(WORKER_REQUIRES_AN_EXPLICIT_SUPPOR.into()))?;
    let request: serde_json::Value =
        serde_json::from_str(&input.0).map_err(|error| Error::Invalid(error.to_string()))?;
    if request
        .get(PARSER_CONFIGURATION)
        .and_then(serde_json::Value::as_str)
        != Some(configuration.0)
    {
        return Err(Error::Invalid(WORKER_PARSER_CONFIGURATION_DIFFER.into()));
    }
    let applicability = request
        .get(RULE_APPLICABILITY)
        .and_then(serde_json::Value::as_object)
        .filter(|values| !values.is_empty())
        .ok_or_else(|| Error::Invalid(WORKER_REQUIRES_EXPLICIT_SELECTED.into()))?;
    let registry: serde_json::Value =
        serde_json::from_str(include_str!("../profiles/pinned-rule-phases.json"))
            .map_err(|error| Error::Invalid(error.to_string()))?;
    for (rule, value) in applicability {
        if !matches!(value.as_str(), Some(ALWAYS | ACCEPTED_SYNTAX))
            || registry
                .get(RULES_DIRECTORY)
                .and_then(|rules| rules.get(rule))
                .is_some_and(|canonical| canonical.get(APPLICABILITY) != Some(value))
        {
            return Err(Error::Invalid(WORKER_RULE_APPLICABILITY_DIFFERS.into()));
        }
    }
    Ok(())
}
