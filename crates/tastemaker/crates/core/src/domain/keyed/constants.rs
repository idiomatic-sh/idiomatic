//! The names of the `keyed` concept: what a path to an element of a sequence is written with.

/// What opens an element's place in a path: `tools.[0]`.
pub(crate) const ELEMENT_OPEN: char = '[';

/// What closes an element's place in a path.
pub(crate) const ELEMENT_CLOSE: char = ']';

/// What a table or a value of the file's no template's meets is named with, aside: none of the template's is.
pub(crate) const ASIDE: char = '~';
