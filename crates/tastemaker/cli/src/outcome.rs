//! How a command ended, and the process exit code that says so; and, in [`remedy`], what
//! the user does next about each error a command can end with. Every match here names
//! every variant: a wildcard is refused, so a new error variant is classed where it is
//! added.
#![deny(clippy::wildcard_enum_match_arm)]

pub mod remedy;

#[cfg(test)]
#[path = "outcome_test.rs"]
mod tests;

/// How a command ended: every command's `run` returns one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub enum Outcome {
    /// The command did its job: exit 0.
    Success,
    /// The command refused its input or reached a negative verdict: exit 1.
    Failure,
    /// The command could not run: exit 2.
    Error,
}

impl Outcome {
    /// The process exit code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Failure => 1,
            Self::Error => 2,
        }
    }
}

/// How an answer ends the command that prints it: a report that holds a negative verdict ends as a failure.
pub trait Ending {
    /// The outcome the command ends with once the answer is printed.
    fn ending(&self) -> Outcome;
}

impl From<Outcome> for std::process::ExitCode {
    fn from(outcome: Outcome) -> Self {
        Self::from(outcome.code())
    }
}
