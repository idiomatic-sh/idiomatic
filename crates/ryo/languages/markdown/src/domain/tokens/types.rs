//! The types of the `tokens` concept: plain data, public fields and derives, with no constructor and no inherent impl. A token is a token type of micromark, named by the type in kebab case.

/// What a token of a Markdown syntax tree is. One closed list, each name once.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    strum::EnumIter,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u16)]
pub(crate) enum Token {
    /// Spaces, tabs and ends of lines between the events of the parser: `whitespace`.
    Whitespace,
    /// The end of a line inside a paragraph: `line-ending`.
    LineEnding,
    /// A line of a code block's code: `code-flow-value`.
    CodeFlowValue,
    /// A line of a block of raw HTML: `html-flow-data`.
    HtmlFlowData,
}
