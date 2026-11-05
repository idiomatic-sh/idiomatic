//! Parse the child's request transport without putting large JSON in argv.
use super::constants;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::path::PathBuf;

/// Text supplied as one executable command argument.
pub(super) struct BenchmarkArgument(pub String);

/// Read an explicit request file or the inline JSON debug argument.
pub(super) fn worker_input(
    args: &mut impl Iterator<Item = BenchmarkArgument>,
) -> Result<String, Error> {
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
        std::fs::read_to_string(path).map_err(Into::into)
    } else {
        Ok(input_argument)
    }
}

/// Bind the executable's parser and phase choices to the actual explicit request.
///
/// # Errors
///
/// Missing, unsupported or conflicting target/phase identities are not executions
/// of a canonical staged profile.
pub(super) fn checked_worker_identity(
    input: &str,
    configuration: Option<&str>,
) -> Result<(), Error> {
    let configuration = configuration
        .filter(|value| super::targets::resolve(Some(*value)).is_ok())
        .ok_or_else(|| {
            Error::Invalid("worker requires an explicit supported parser configuration".into())
        })?;
    let request: serde_json::Value =
        serde_json::from_str(input).map_err(|error| Error::Invalid(error.to_string()))?;
    if request
        .get("parser_configuration")
        .and_then(serde_json::Value::as_str)
        != Some(configuration)
    {
        return Err(Error::Invalid(
            "worker parser configuration differs from its profile identity".into(),
        ));
    }
    let applicability = request
        .get("rule_applicability")
        .and_then(serde_json::Value::as_object)
        .filter(|values| !values.is_empty())
        .ok_or_else(|| {
            Error::Invalid("worker requires explicit selected rule applicability".into())
        })?;
    let registry: serde_json::Value =
        serde_json::from_str(include_str!("../profiles/pinned-rule-phases.json"))
            .map_err(|error| Error::Invalid(error.to_string()))?;
    for (rule, value) in applicability {
        if !matches!(value.as_str(), Some("always" | "accepted-syntax"))
            || registry
                .get("rules")
                .and_then(|rules| rules.get(rule))
                .is_some_and(|canonical| canonical.get("applicability") != Some(value))
        {
            return Err(Error::Invalid(
                "worker rule applicability differs from the pinned phase policy".into(),
            ));
        }
    }
    Ok(())
}
