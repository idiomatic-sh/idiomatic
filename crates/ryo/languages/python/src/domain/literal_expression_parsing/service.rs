//! Reconstruct preserved logical string carriers and cache only syntactic outcomes.
use super::constants;
use super::types::{LiteralParseOutcome};

use ryo::languages::{NameText};


/// Missing logical carriers must remain observable through a partial relation read.
pub(crate) const INCOMPLETE: &str = constants::INCOMPLETE;

/// Translate an application-owned outcome into the stable relation vocabulary.
pub(crate) const fn outcome_name(outcome: LiteralParseOutcome) -> NameText<'static> {
    match outcome {
        LiteralParseOutcome::Parsed => NameText(constants::PARSED),
        LiteralParseOutcome::Malformed => NameText(constants::MALFORMED),
        LiteralParseOutcome::Unavailable => NameText(constants::UNAVAILABLE),
    }
}
