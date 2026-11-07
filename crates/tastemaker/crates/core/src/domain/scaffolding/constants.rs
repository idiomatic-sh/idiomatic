//! The names of the `scaffolding` concept: the text its code looks for.

/// What separates a module file's name from its extension: `src/inbound` is declared by `src/inbound.<ext>`.
pub(crate) const EXTENSION_MARK: char = '.';

/// What opens a regex group, before `P<name>` or `<name>` for a named one.
pub(crate) const GROUP_OPEN: &str = "(?";

/// Separates explanations returned by independent condition queries.
pub(crate) const CONDITION_SEPARATOR: &str = "; ";
