//! Vocabulary of optional physical token edges.
use super::item::TokenLinkColumn;
use crate::domain::relations::constants::SOURCE;
/// Canonical optional relation name.
#[must_use]
pub fn name() -> &'static str {
    "token-link"
}
/// Structural promise, without lint policy.
#[must_use]
pub fn summary() -> &'static str {
    "Physical native token successor, matching and immediate enclosing delimiter edges; absent links are omitted, never sentinel nodes."
}
/// One public typed-column name.
#[must_use]
pub fn column_name(column: TokenLinkColumn) -> &'static str {
    match column {
        TokenLinkColumn::Source => SOURCE,
        TokenLinkColumn::Target => "target",
        TokenLinkColumn::LinkKind => "link-kind",
        TokenLinkColumn::File => "file",
        TokenLinkColumn::Span => "span",
    }
}
