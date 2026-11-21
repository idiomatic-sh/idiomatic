//! The types of the `asks` part.

use crate::domain::code::types::CodeEntry;

/// What an `Exists` word's quick ask of its body came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QuickAsk {
    /// The answer is in the state: nothing is run.
    Answered,
    /// The program at the entry is to be run where the word stands, for whether it has a row.
    Run(CodeEntry),
    /// The body is asked as any body is (`bodies::service::exists`).
    Kept,
}

/// SPIKE: whether a memo site's answers are kept for the running rule in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SitePasses {
    /// Kept by key, and looked for.
    Kept,
    /// Not kept: each ask is evaluated, and its answer lasts until the next.
    Passing,
}
