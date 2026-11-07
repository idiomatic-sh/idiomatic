//! The types of the `engine` operation: why the kind check refused a request's rules.
use ryo_execution::inbound::use_cases::metadata::interface::{KindMessage, RuleIndex};

/// A rule that writes a word for a kind no linked language has, and why it is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KindRefused {
    /// The rule that writes the word.
    pub rule: RuleIndex,
    /// What the kind check says of the word.
    pub reason: KindMessage,
}
