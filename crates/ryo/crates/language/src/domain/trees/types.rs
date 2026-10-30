//! The types of the `trees` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::elements::types::TableBuilder;
use crate::domain::relations::types::Offset;

/// A tree written from a library's events and the text between them: the builder it is written through, and where the last token
/// it wrote ends. Every byte before `end` is in one token; the text no event spans is written as the tokens of the language's gap
/// tokenizer before the next event, so the tokens read back the whole file.
#[derive(Debug)]
pub struct Lossless<'b, 'w> {
    /// What the tree is written through.
    pub builder: &'b mut TableBuilder<'w>,
    /// Where the last token written ends.
    pub end: Offset,
}

/// The text of a file its library reads: its bytes up to the first that is not UTF-8. The bytes after it, when there are any, are text
/// the library could not read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadableText<'a>(pub &'a str);
