//! The constants of the part: the text its code uses.

/// Why a root is not complete when the physical source facts behind it are not.
pub(crate) const INCOMPLETE_FACTS: &str =
    crate::domain::recording::constants::PHYSICAL_FACTS_INCOMPLETE;
/// Why a demanded observation phase cannot publish.
pub(crate) const NO_RESERVED_ARENA: &str =
    "demanded observation phase has no reserved publication arena";
