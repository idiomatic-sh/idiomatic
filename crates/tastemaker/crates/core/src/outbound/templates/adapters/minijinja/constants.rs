//! The filters and functions a template's content may use, beside minijinja's own.

/// `UpperCamelCase`.
pub(super) const PASCAL: &str = "pascal";
/// `lowerCamelCase`.
pub(super) const CAMEL: &str = "camel";
/// `snake_case`.
pub(super) const SNAKE: &str = "snake";
/// `kebab-case`.
pub(super) const KEBAB: &str = "kebab";
/// `SHOUTY_SNAKE_CASE`.
pub(super) const SHOUTY: &str = "shouty";
/// `refuse("why")`: the template renders nothing and says why, as when a first writing needs a value nothing gave.
pub(super) const REFUSE: &str = "refuse";
/// What joins the words of a param's name as it is written, `item-trait`, which a message says it by.
pub(super) const KEBAB_TEXT: &str = "-";
