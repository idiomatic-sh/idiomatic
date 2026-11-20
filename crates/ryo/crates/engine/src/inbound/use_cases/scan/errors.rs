//! Why `scan` faults.

use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::{Missing, Relation};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::stored_facts::types::PartialCause;

/// Why a file's scan stopped for a rule: a fault of the rules whose enabled programs read the relation, never empty rows.
///
/// # Examples
///
/// ```
/// use ryo_engine::inbound::use_cases::scan::errors::ScanError;
/// use ryo_relations::domain::answers::types::AbsenceCause;
/// use ryo_relations::domain::relations::types::Relation;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// let refused = ScanError::Refused { rule: SelectedRuleOrdinal(0), relation: Relation::Function, cause: AbsenceCause::NotCode };
/// assert!(refused.to_string().contains("refused"));
/// ```
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ScanError {
    /// The language refused a relation a candidate of the rule needs.
    #[error("rule {rule:?}: relation {relation:?} refused: {cause:?}")]
    Refused {
        /// The rule.
        rule: SelectedRuleOrdinal,
        /// The relation.
        relation: Relation,
        /// Why the language refused it.
        cause: AbsenceCause,
    },
    /// The language filled a relation a candidate of the rule needs only in part.
    #[error("rule {rule:?}: relation {relation:?} read in part: {cause:?}")]
    Partial {
        /// The rule.
        rule: SelectedRuleOrdinal,
        /// The relation.
        relation: Relation,
        /// The language's sentence.
        cause: PartialCause,
        /// Which rows may be missing.
        missing: Missing,
    },
}
