//! Why `project` faults.

use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// Why the project pass stopped for a rule.
///
/// # Examples
///
/// ```
/// use ryo_engine::inbound::use_cases::project::errors::ProjectPassError;
/// use ryo_relations::domain::answers::types::AbsenceCause;
/// use ryo_relations::domain::relations::types::Relation;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// let refused = ProjectPassError::Refused { rule: SelectedRuleOrdinal(0), relation: Relation::ModuleIdentity, cause: AbsenceCause::NoManifests };
/// assert!(refused.to_string().contains("refused"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ProjectPassError {
    /// The project refused a relation a program of the rule needs.
    #[error("rule {rule:?}: project relation {relation:?} refused: {cause:?}")]
    Refused {
        /// The rule.
        rule: SelectedRuleOrdinal,
        /// The relation.
        relation: Relation,
        /// Why the project refused it.
        cause: AbsenceCause,
    },
}
