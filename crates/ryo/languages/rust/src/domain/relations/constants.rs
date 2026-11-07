//! The names of the `relations` concept: the text its rows carry when they leave the engine.

use ryo::languages;

/// The name the receiver is bound to.
pub(crate) const SELF_VALUE: &str = "self";

/// The kind of the rows `defines` writes for a name a `let` binds.
pub(crate) const VARIABLE: languages::RowKind = languages::RowKind("variable");

/// Why `defines` is only partly read in a file with a name in a destructuring `let` that starts as a constant does.
pub(crate) const NAME_LIKE_CONSTANT: &str =
    "a name in a destructuring `let` that starts as a constant does may be one, and not a variable";
