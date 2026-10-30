//! The types of the `ra_ap_syntax` adapter: plain data, public fields and derives, with no constructor and no inherent impl.

use ra_ap_syntax::SyntaxKind;
use ryo::languages::{Count, Offset, Position};

/// Rust source text as the adapter reads it: the file's text, with a character the lexer reports in place of each byte that is not part
/// of one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RustText<'a>(pub &'a str);

/// The text of a file as the adapter reads it, borrowed from the file's bytes when they are text and made otherwise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Repaired<'a>(pub std::borrow::Cow<'a, str>);

/// A string literal as it is written, quotes and escapes and all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StringLiteral<'a>(pub &'a str);

/// The value a string literal stands for, with its escapes resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DecodedString(pub String);

/// What the parser said of text it could not read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ParserMessage(pub Box<str>);

impl std::ops::Deref for ParserMessage {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The name of a macro, as it is written before the `!`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MacroName(pub Box<str>);

impl std::ops::Deref for MacroName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// One step of the parser's events, owned, so a token tree can be held while it is read again.
#[derive(Clone, Debug)]
pub(crate) enum Step {
    /// A node is entered.
    Enter(SyntaxKind),
    /// A token of this many bytes.
    Token(SyntaxKind, Count),
    /// The node last entered is left.
    Exit,
    /// The parser could not read the text at this offset.
    Error(ParserMessage, Offset),
}

/// Where the writing of steps into the table has come to.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cursor {
    /// The byte the next token starts at.
    pub position: Offset,
    /// The place the next element has in the table.
    pub next: Position,
}

/// A direct child of a token tree: the bytes it spans, and its kind.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Child {
    /// Where it starts.
    pub from: Offset,
    /// Where it ends.
    pub to: Offset,
    /// Its kind (that of its node, or of its token).
    pub kind: SyntaxKind,
}

/// A stretch of text in an argument list.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ArgumentSpan {
    /// Where it starts.
    pub from: Offset,
    /// Where it ends.
    pub to: Offset,
}

/// What a stretch of an argument list is read as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reading {
    /// An expression.
    Expression,
    /// A pattern.
    Pattern,
}

/// A stretch of an argument list and what it is read as.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Stretch {
    /// The text.
    pub span: ArgumentSpan,
    /// What it is read as.
    pub reading: Reading,
}

/// An argument of a macro read again: where its text starts and the steps it parses to.
#[derive(Clone, Debug)]
pub(crate) struct MacroArgument {
    /// Where its text starts.
    pub from: Offset,
    /// The steps it parses to.
    pub steps: Vec<Step>,
}

/// A node being open while the file's own tree is written.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Open {
    /// The node's place in the table.
    pub at: Position,
}

/// A token tree being held while its macro is looked at.
#[derive(Clone, Debug)]
pub(crate) struct Held {
    /// The macro it is the arguments of.
    pub name: MacroName,
    /// The place in the table of the node that holds it.
    pub host: Position,
    /// Where its text starts.
    pub start: Offset,
    /// How many nodes of it are open.
    pub depth: Count,
    /// The steps taken so far.
    pub steps: Vec<Step>,
}

/// The arguments of a macro call read again, to be added under the call.
#[derive(Clone, Debug)]
pub(crate) struct Expansion {
    /// The place in the table of the call.
    pub host: Position,
    /// The arguments.
    pub arguments: Vec<MacroArgument>,
}

/// Whether a child of a token tree is white space or a comment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Triviality {
    /// It is.
    Trivia,
    /// It is not.
    Significant,
}
