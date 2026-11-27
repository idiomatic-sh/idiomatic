//! The types of the `swc_html_parser` adapter: plain data, public fields and derives, with no constructor and no inherent impl.

/// The parser's input with its actual tokenizer output retained for source reconstruction.
pub(super) struct Recording<'a, I> {
    /// The tokenizer the parser reads through it.
    pub(crate) input: I,
    /// Every token the tokenizer delivered, in the order it delivered them.
    pub(crate) tokens: &'a mut Vec<swc_html_ast::TokenAndSpan>,
}

/// A native DOM scalar, separate from a child edge.
pub(super) struct PropertyValue<'a> {
    /// Its property identity.
    pub(crate) key: crate::domain::syntax::types::Property,
    /// Its decoded native value.
    pub(crate) value: ryo::languages::NameText<'a>,
}
/// The number of source bytes consumed.
#[derive(Clone, Copy, Debug)]
pub(super) struct SourceLength(pub usize);
