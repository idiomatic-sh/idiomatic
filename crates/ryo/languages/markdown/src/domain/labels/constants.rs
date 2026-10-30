//! The names of the `labels` concept: the characters and words of the raw HTML it reads.

/// What opens a tag.
pub(crate) const TAG_OPEN: char = '<';

/// What closes a tag.
pub(crate) const TAG_CLOSE: char = '>';

/// What a closing tag, a comment and a declaration open with, after the `<`: none of them has attributes to read.
pub(crate) const NOT_AN_OPENING_TAG: [char; 3] = ['/', '!', '?'];

/// What ends a tag's name and an attribute's name.
pub(crate) const NAME_END: [char; 3] = ['=', '/', '>'];

/// What sets an attribute's value after its name.
pub(crate) const EQUALS: char = '=';

/// What can end the white space before an attribute: a slash of a tag that closes itself.
pub(crate) const SLASH: char = '/';

/// The quotes a value is written between.
pub(crate) const QUOTES: [char; 2] = ['"', '\''];

/// The attribute whose value is an anchor, on any tag.
pub(crate) const ID: &str = "id";

/// The attribute whose value is an anchor, on an `a`.
pub(crate) const NAME: &str = "name";

/// The tag whose `name` is an anchor.
pub(crate) const ANCHOR_TAG: &str = "a";

/// The value of an attribute written with none.
pub(crate) const NO_VALUE: &str = "";
