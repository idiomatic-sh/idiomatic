//! The names of the `prose` concept.

use ryo::languages::NameText;

/// What separates two words of a run.
pub(crate) const WORD_SPACE: NameText<'static> = NameText(" ");

/// What opens and closes a code span.
pub(crate) const CODE_FENCE: char = '`';
