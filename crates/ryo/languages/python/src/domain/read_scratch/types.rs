//! The values of the readers' retained buffers.
use crate::domain::format_specification::types::Observations;
use crate::domain::token_links::types::Opener;
use ryo::languages::Position;
use std::collections::HashSet;

/// Buffers a reader empties at each use, so a file that fits what an earlier one grew allocates nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ReadScratch {
    /// The embedded-expression hosts `completion_hosts` skips.
    pub completion_expanded: HashSet<Position>,
    /// The unmatched openers of the token-link scan.
    pub token_stack: Vec<Opener>,
    /// The format-specification observations, with their text and scratch.
    pub format_observations: Observations,
}
