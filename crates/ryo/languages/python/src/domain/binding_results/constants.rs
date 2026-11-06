//! Stable generic static-binding vocabulary.
/// Incomplete frontend analysis must never masquerade as a proven missing name.
pub(crate) const INCOMPLETE: &str = "binding analysis requires complete supported syntax";
/// The text `wildcard`.
pub(crate) const WILDCARD: &str = "wildcard";
/// The text `read`.
pub(crate) const READ: &str = "read";
/// The text `bind`.
pub(crate) const BIND: &str = "bind";
/// The text `delete`.
pub(crate) const DELETE: &str = "delete";
/// The text `cleanup`.
pub(crate) const CLEANUP: &str = "cleanup";
/// The text `global`.
pub(crate) const GLOBAL: &str = "global";
/// The text `nonlocal`.
pub(crate) const NONLOCAL: &str = "nonlocal";
/// The text `unique-declaration`.
pub(crate) const UNIQUE_DECLARATION: &str = "unique-declaration";
/// The text `alternative-declarations`.
pub(crate) const ALTERNATIVE_DECLARATIONS: &str = "alternative-declarations";
/// The text `possibly-unbound`.
pub(crate) const POSSIBLY_UNBOUND: &str = "possibly-unbound";
/// The text `unbound`.
pub(crate) const UNBOUND: &str = "unbound";
/// The text `builtin`.
pub(crate) const BUILTIN: &str = "builtin";
/// The text `declaration-or-builtin`.
pub(crate) const DECLARATION_OR_BUILTIN: &str = "declaration-or-builtin";
/// The text `uncertain`.
pub(crate) const UNCERTAIN: &str = "uncertain";
/// The text `value`.
pub(crate) const VALUE: &str = "value";
/// The text `type`.
pub(crate) const TYPE: &str = "type";
/// The text `deletion`.
pub(crate) const DELETION: &str = "deletion";
/// The text `unique`.
pub(crate) const UNIQUE: &str = "unique";
/// The text `alternative`.
pub(crate) const ALTERNATIVE: &str = "alternative";
/// The text `replacement`.
pub(crate) const REPLACEMENT: &str = "replacement";
/// The text `lexical`.
pub(crate) const LEXICAL: &str = "lexical";
