//! Execution attribution reads the request-owned exact rule identity.
use super::constants::PLACE_OF_RULE;
use crate::domain::coverage::types::RuleIndex;
use ryo_values::domain::runs::types::RuleId;
/// The id of the rule at `place`.
///
/// # Panics
///
/// On a place past the rules: the places come only from the rules of the run, so one past them is a bug.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "a rule's place comes only from the rules of the run; one past them is a bug that must crash, not a blank id that hides the rule"
)]
pub(crate) fn id_at(ids: &[RuleId], place: RuleIndex) -> RuleId {
    ids.get(place.0).cloned().expect(PLACE_OF_RULE)
}
