//! The types of the `ra_ap_syntax` adapter: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::outbound::syntax::types::OpaqueMacros;
use ra_ap_syntax::SyntaxKind;
use ryo::languages::{Count, Flag, Offset, Position};

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

/// The name of a macro, as it is written before the `!`: the last segment of the call's path.
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
    /// Whether the node last entered is a literal, whose numeric token gives it a value.
    pub literal: Flag,
}

/// The grammar a macro call's arguments are read by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArgumentGrammar {
    /// A list of expressions separated by commas, a trailing comma allowed, each of them perhaps named (`width = 3`, read as its value):
    /// the arguments of every macro but `vec!` and `matches!`.
    Arguments,
    /// The elements of an array (`vec![a, b]`) or an element and its count (`vec![a; n]`).
    Vector,
    /// An expression, then a pattern and perhaps `if` and a guard (`matches!`).
    Matches,
}

/// An argument of a macro read again: where its text starts and the steps it parses to.
#[derive(Clone, Debug)]
pub(crate) struct MacroArgument {
    /// Where its text starts.
    pub from: Offset,
    /// The steps it parses to.
    pub steps: Vec<Step>,
}

/// A node being open while a tree is written.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Open {
    /// The node's place in the table.
    pub at: Position,
    /// Its kind: a token tree is a call's arguments only when it is a child of a macro call.
    pub kind: SyntaxKind,
}

/// A token tree being held while its macro is looked at.
#[derive(Clone, Debug)]
pub(crate) struct HeldMacro {
    /// The macro it is the arguments of.
    pub name: MacroName,
    /// The place in the table of the node that holds it.
    pub host: Position,
    /// Where its text starts.
    pub start: Offset,
    /// How many nodes of it are open.
    pub depth: Count,
    /// The kind of its first token, when the token tree starts with one: its opening delimiter.
    pub opening: Option<SyntaxKind>,
    /// The kind of the last step written directly under it, when that step is a token: its closing delimiter, once it is whole.
    pub closing: Option<SyntaxKind>,
}

/// The arguments of a macro call read again, to be added under the call.
#[derive(Clone, Debug)]
pub(crate) struct Expansion {
    /// The place in the table of the call.
    pub host: Position,
    /// The arguments.
    pub arguments: Vec<MacroArgument>,
}

/// The text an argument list is parsed from: the token tree, its delimiters replaced by those of a tuple or an array.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ArgumentList(pub String);

/// The direct children of a node: each child node, with where its text starts, and the kinds of the tokens directly under it that are
/// not white space or comments.
#[derive(Clone, Debug, Default)]
pub(crate) struct NodeChildren {
    /// The child nodes, in order.
    pub nodes: Vec<MacroArgument>,
    /// The kinds of the tokens directly under it, white space and comments aside.
    pub tokens: Vec<SyntaxKind>,
}

/// Where a walk of a tree has come to, for finding the macro calls in it: the file's own tree, or the arguments read again of a call.
#[derive(Debug)]
pub(crate) struct Walker {
    /// The built-in macros whose arguments are never read again.
    pub opaque: OpaqueMacros<'static>,
    pub revision: crate::domain::model::types::ProviderRevision,
    /// Where the last name written starts.
    pub name_from: Offset,
    /// Where the last name written ends.
    pub name_to: Offset,
    /// Whether the token tree that follows is the arguments of a macro call: the last token was the `!` of a call.
    pub armed: Flag,
    /// The nodes open, with the place each has in the table.
    pub open: Vec<Open>,
    /// The token tree of a call being written, held until it is whole.
    pub held: Option<HeldMacro>,
    /// The calls whose arguments were read, in the order their token trees closed.
    pub found: Vec<Expansion>,
    /// The text an argument list is parsed from, kept between calls so it is made once.
    pub buffer: ArgumentList,
    /// The top-level token tree for the 0.1.4 fragment grammar, reused between calls.
    pub fragment_steps: Vec<Step>,
}
