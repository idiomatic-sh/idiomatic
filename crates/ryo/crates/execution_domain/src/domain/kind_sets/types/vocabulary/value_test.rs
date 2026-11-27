//! Literal-mask setup work, exact vocabulary identity and generation isolation.

/// Prepared metadata can cross worker ownership boundaries without retaining local source borrows.
#[test]
fn prepared_vocabulary_is_send_and_sync() {
    fn requires_send_sync<T: Send + Sync>() {}
    requires_send_sync::<crate::domain::kind_sets::types::vocabulary::value::PreparedVocabulary>();
}
