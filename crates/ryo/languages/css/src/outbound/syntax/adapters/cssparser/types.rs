//! The types of the `cssparser` adapter: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::nodes::types::CssNode;
use ryo::languages::ReportText;

/// What the block of an at-rule holds: rules and declarations, as a style rule's does (`@media`, `@font-face`), or component
/// values, for an at-rule the language does not know.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Body {
    /// Rules and declarations.
    Rules,
    /// Component values.
    Components,
}

/// What the adapter makes of a token the parser gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Met {
    /// White space or a comment.
    Trivia,
    /// A token that opens a function or a simple block: the node it opens.
    Opens(CssNode),
    /// A token the library says is a parse error, and what the language says of it.
    Broken(ReportText<'static>),
    /// Any other token of content.
    Content,
}

/// Whether a declaration's value holds a `{}` block beside other content: then it is no declaration, and is read again as a rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Contents {
    /// It does not.
    Plain,
    /// It does.
    HoldsBlock,
}

/// How many of the standard's tokens a token of the library is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pieces {
    /// One.
    Whole,
    /// Two delim-tokens of one byte each: an attribute matcher, `~=`, which the library reads as one token.
    TwoDelims,
}

/// Where an open node ends: at its last component value, or, once the `{` of a rule's body is read, where the parser is when the
/// library finishes the rule (after its `}`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Closing {
    /// At its content.
    AtContent,
    /// Where the parser is.
    AtPlace,
}

/// A node the adapter has open, and where it ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Open {
    /// The node.
    pub node: CssNode,
    /// Where it ends.
    pub closing: Closing,
}

/// A place in the text the parser reads, in bytes from its start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ByteIndex(pub usize);

/// The text the parser reads: the file after its byte order mark, up to its first byte that is not UTF-8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CssText<'a>(pub &'a str);

/// The part of the text a rule or a declaration the library recovered from spans, as the library hands it back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Recovered<'a>(pub &'a str);

/// The text of a message the adapter writes, kept for the length of one file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Scratch(pub String);
