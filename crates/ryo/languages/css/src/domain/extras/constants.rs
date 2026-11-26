//! The names of the `extras` concept: why the relations read none of most of them.

/// What the decoding drops.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const DECODING: &str =
    "a byte order mark, which the decoding drops: `syntax_tree` holds it as it stands";
