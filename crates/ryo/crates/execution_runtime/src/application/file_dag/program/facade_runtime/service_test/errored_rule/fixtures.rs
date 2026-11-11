//! The support of the `errored_rule` cases.
use super::super::*;
use ryo_values::domain::query_tokens::types::QuerySourceText;
use ryo_values::domain::runs::types::RuleId;

/// A rule written as a user writes it: an id and its query text.
pub(super) fn written_rule(
    id: &str,
    text: &str,
) -> ryo_values::domain::rule_requests::types::RuleSpec {
    let mut spec = rule();
    spec.id = RuleId(id.into());
    spec.query.root.text = QuerySourceText(text.into());
    spec
}
