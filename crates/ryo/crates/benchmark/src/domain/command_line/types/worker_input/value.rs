//! Where one worker request is read from, read without putting large JSON in argv.
use super::super::super::constants::MISSING_WORKER_REQUEST;
use super::super::{BenchmarkArgument, InlineRequest};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::constants::REQUEST_FILE;
use std::path::PathBuf;

/// Where one worker request is read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkerInput {
    /// A request file the parent owns.
    File(PathBuf),
    /// The inline JSON debug argument.
    Inline(InlineRequest),
}

impl WorkerInput {
    /// Name an explicit request file or the inline JSON debug argument.
    ///
    /// # Errors
    /// Refuses a command line without a request, or `--request-file` without its path.
    ///
    /// # Examples
    /// ```
    /// use ryo_benchmark::domain::command_line::types::{worker_input::value::WorkerInput, BenchmarkArgument};
    /// let mut arguments = ["--request-file", "request.json"].map(|a| BenchmarkArgument(a.into())).into_iter();
    /// assert_eq!(WorkerInput::read(&mut arguments)?, WorkerInput::File("request.json".into()));
    /// # Ok::<(), ryo_benchmark_contracts::domain::reports::errors::ReportsError>(())
    /// ```
    pub fn read(args: &mut dyn Iterator<Item = BenchmarkArgument>) -> Result<Self, Error> {
        let input_argument = args
            .next()
            .map(|argument| argument.0)
            .ok_or_else(|| Error::Invalid(MISSING_WORKER_REQUEST.into()))?;
        if input_argument == REQUEST_FILE {
            args.next()
                .map(|argument| argument.0)
                .map(PathBuf::from)
                .map(Self::File)
                .ok_or_else(|| Error::Invalid(MISSING_WORKER_REQUEST.into()))
        } else {
            Ok(Self::Inline(InlineRequest(input_argument)))
        }
    }
}
