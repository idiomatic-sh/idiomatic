//! The types of the `tools` port: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::manifest::types::{Argv, ProductRoot, Seconds};

/// One run asked of a tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invocation {
    /// The command line.
    pub argv: Argv,
    /// Where it runs: the product's root, so a tool's own configuration resolves the same whatever is checked.
    pub directory: ProductRoot,
    /// How long it may run.
    pub timeout: Seconds,
}

/// A process's exit code.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExitCode(pub i32);

/// Why a tool could not start, said for the user.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StartProblem(pub Box<str>);

/// How a run ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ending {
    /// It exited with this code.
    Exited(ExitCode),
    /// A signal ended it, which is never how a linter finishes.
    Signalled,
    /// It ran past its timeout, and everything it started was killed.
    TimedOut,
    /// It could not start.
    Unstartable(StartProblem),
}

/// What a run wrote to one stream, as text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Captured(pub Box<str>);

/// A finished run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    /// How it ended.
    pub ending: Ending,
    /// What it wrote to stdout.
    pub stdout: Captured,
    /// What it wrote to stderr.
    pub stderr: Captured,
}

/// The process group a running tool leads: its leader's process id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupId(pub u32);
