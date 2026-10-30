//! The fake of the `tools` port: each program's answer given up front, and every run recorded.

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::domain::manifest::types::{Argument, Argv};
use crate::outbound::tools::interface::ToolsPort;
use crate::outbound::tools::types::{
    Captured, Completion, Ending, ExitCode, Invocation, StartProblem,
};

/// What the core's tests stand in for `tools` with.
#[derive(Debug, Default)]
pub struct ToolsFake {
    /// What each program answers, by its name.
    pub answers: BTreeMap<Argument, Completion>,
    /// The command lines run, in order.
    pub runs: Mutex<Vec<Argv>>,
}

impl ToolsFake {
    /// The fake with `program` answering exit `code` and `stdout`.
    #[must_use]
    pub fn answering(mut self, program: &str, code: i32, stdout: &str) -> Self {
        // A later answer for the same program replaces the earlier one.
        self.answers.extend([(
            Argument(program.into()),
            Completion {
                ending: Ending::Exited(ExitCode(code)),
                stdout: Captured(stdout.into()),
                stderr: Captured(Box::default()),
            },
        )]);
        self
    }

    /// The command lines run so far.
    #[must_use]
    pub fn ran(&self) -> Vec<Argv> {
        self.runs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl ToolsPort for ToolsFake {
    fn run(&self, invocation: &Invocation) -> Completion {
        self.runs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(invocation.argv.clone());
        invocation
            .argv
            .0
            .first()
            .and_then(|program| self.answers.get(program))
            .cloned()
            .unwrap_or_else(|| Completion {
                ending: Ending::Unstartable(StartProblem("no such program".into())),
                stdout: Captured::default(),
                stderr: Captured::default(),
            })
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
