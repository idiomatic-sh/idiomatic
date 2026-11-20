//! The types of the `terminal_facts` concept: how a statement or a run of statements ends.

use crate::domain::terminal_flow::types::{Breaks, TerminalOutcome};

/// How a statement, or a run of statements, ends: its terminal outcome, and whether a `break` in it can leave the loop around it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Ending {
    /// The terminal outcome.
    pub terminal: TerminalOutcome,
    /// Whether a `break` can leave the loop around it.
    pub breaks: Breaks,
}

/// Whether one of a statement's alternatives always runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AlternativeCoverage {
    /// One alternative always runs: an `else`, or a case that takes every subject.
    Exhaustive,
    /// No alternative may run.
    Partial,
}
