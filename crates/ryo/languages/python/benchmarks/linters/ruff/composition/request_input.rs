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

#[cfg(all(test, not(bench)))]
mod request_file_tests {
    use super::{BenchmarkArgument, worker_input};

    #[test]
    fn reads_the_exact_explicit_request_file() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let request_path = directory.path().join("request.json");
        let request = r#"{"root":"/tmp/project","files":["a.py"]}"#;
        std::fs::write(&request_path, request)?;
        let mut arguments = [
            BenchmarkArgument("--request-file".into()),
            BenchmarkArgument(request_path.to_string_lossy().into_owned()),
        ]
        .into_iter();
        assert_eq!(worker_input(&mut arguments)?, request);
        assert!(arguments.next().is_none());
        Ok(())
    }

    #[test]
    fn retains_the_explicit_inline_json_debug_protocol() -> Result<(), Box<dyn std::error::Error>> {
        let request = r#"{"root":"/tmp/project","files":["a.py"]}"#;
        let mut arguments = [BenchmarkArgument(request.into())].into_iter();
        assert_eq!(worker_input(&mut arguments)?, request);
        assert!(arguments.next().is_none());
        Ok(())
    }
}
