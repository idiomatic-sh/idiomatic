//! The types of the `comments` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::findings::types::ByteOffset;

/// A comment's content, as the `comment` relation gives it: without its delimiters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommentText<'a>(pub &'a str);

/// A range of bytes of a file's text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteSpan {
    /// The first byte.
    pub start: ByteOffset,
    /// The byte after the last.
    pub end: ByteOffset,
}

/// What a comment is.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommentKind {
    /// A line comment that is not documentation, with what follows its marker.
    Line(CommentBody),
    /// Documentation or a block.
    Other,
}

/// Whether a comment is documentation, as the language that read it says: the shared `comment` relation's `doc`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Documentation {
    /// It documents the code it is written with, as the language that read it marks its documentation.
    Documents,
    /// It is any other comment.
    Plain,
}

/// Whether the line a finding is shown on passes over a comment its node starts with: documentation and an ignore are about
/// the code below them; any other comment is shown as written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Passing {
    /// The shown line passes over it.
    Passed,
    /// The shown line may be its line.
    Kept,
}

/// One comment of a file.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Comment {
    /// Where it is.
    pub span: ByteSpan,
    /// What it is.
    pub kind: CommentKind,
    /// Whether a finding's shown line passes over it.
    pub passing: Passing,
}

/// A line of a file, from one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineIndex(pub u32);

/// Whether a line of a file holds code: a non-blank character outside every comment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineCode {
    /// It does.
    Code,
    /// Only comments and blank space.
    Blank,
}

/// A file as its comments leave it: where each line starts, which lines hold code, and the byte ranges of its comments.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceLines {
    /// Where each line starts.
    pub starts: Vec<ByteOffset>,
    /// Whether each line holds code.
    pub code: Vec<LineCode>,
    /// The comments' byte ranges, sorted.
    pub comments: Vec<ByteSpan>,
    /// The byte ranges of the comments a finding's shown line passes over, sorted.
    pub passed: Vec<ByteSpan>,
}

/// What follows a line comment's marker.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommentBody(pub Box<str>);

impl std::ops::Deref for CommentBody {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

/// Whether a byte of a file lies within one of its comments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Commentary {
    /// It does.
    Commented,
    /// It lies in code or blank space.
    Uncommented,
}
