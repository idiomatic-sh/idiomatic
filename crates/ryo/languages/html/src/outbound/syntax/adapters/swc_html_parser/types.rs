//! The types of the `swc_html_parser` adapter: plain data, public fields and derives, with no constructor and no inherent impl.

/// The parser's input with its actual tokenizer output retained for source reconstruction.
pub(super) struct Recording<'a, I> {
    /// The tokenizer the parser reads through it.
    pub input: I,
    /// Every token the tokenizer delivered, in the order it delivered them.
    pub tokens: &'a mut Vec<swc_html_ast::TokenAndSpan>,
}

/// A native DOM scalar, separate from a child edge.
pub(super) struct PropertyValue<'a> {
    /// Its property identity.
    pub key: crate::domain::syntax::types::Property,
    /// Its decoded native value.
    pub value: ryo::languages::NameText<'a>,
}
/// The properties of a node as the one JSON text the adapter writes them in: each property's number and its value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Published(pub String);

/// The number of source bytes consumed.
#[derive(Clone, Copy, Debug)]
pub(super) struct SourceLength(pub usize);
