//! The request of `evaluate`.

use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_relations::domain::relations::types::FileId;

/// One file to evaluate the rules on.
#[derive(Clone, Copy, Debug)]
pub struct EvaluateInput<'a> {
    /// The file's number.
    pub file: FileId,
    /// The file's bytes.
    pub source: &'a [u8],
    /// What the file asks of each rule, by the rule's place: only a rule that runs is evaluated.
    pub intents: &'a [FileRuleIntent],
}
