//! Parse the child's request transport without putting large JSON in argv.
use super::constants;
use ryo_benchmark::domain::diagnostics::errors::DiagnosticsError as Error;
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
