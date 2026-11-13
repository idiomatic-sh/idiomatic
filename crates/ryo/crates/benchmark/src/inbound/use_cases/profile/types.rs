//! What a bench hands the profile run, and what the run answers.
use crate::domain::command_line::types::BenchmarkArgument;
use crate::domain::profiles::types::ProfileReceipt;
use std::path::PathBuf;

/// A bench's own executable and its command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileRequest {
    /// The bench's executable, which the profiled children run.
    pub worker: PathBuf,
    /// Its arguments, as cargo passed them.
    pub arguments: Vec<BenchmarkArgument>,
}

/// The command the bench runs next, and the profiles recorded before it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileReport {
    /// The bench's command line, cargo's `--bench` removed.
    pub command: Vec<BenchmarkArgument>,
    /// The profiles, when they were asked for.
    pub profile: Profile,
}

/// Whether the command line asked for profiles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Profile {
    /// The command line asked for another command.
    Unasked,
    /// The heap and CPU profiles were recorded.
    Recorded(ProfileReceipt),
}
