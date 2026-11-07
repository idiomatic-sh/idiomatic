//! The types of the `labels` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Raw HTML, or the part of it still to be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HtmlText<'a>(pub &'a str);

/// A tag: what lies between its `<` and its `>`, and the HTML after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Tag<'a> {
    /// What lies between the `<` and the `>`.
    pub inner: HtmlText<'a>,
    /// The HTML after the `>`.
    pub after: HtmlText<'a>,
}

/// The name of an attribute, as written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AttributeName<'a>(pub &'a str);

/// The value of an attribute, without its quotes; empty when it has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AttributeValue<'a>(pub &'a str);

/// An attribute of a tag, and the tag's text after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Attribute<'a> {
    /// Its name.
    pub name: AttributeName<'a>,
    /// Its value.
    pub value: AttributeValue<'a>,
    /// The text of the tag after it.
    pub after: HtmlText<'a>,
}

/// The name of a place a link can point at, as raw HTML gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AnchorText<'a>(pub &'a str);

/// The name of a place a definition gives: the label of a definition of a link reference or of a footnote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LabelText<'a>(pub &'a str);

/// What the element a reader of definitions is given defines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Defined<'a> {
    /// A section, which a heading opens.
    Section,
    /// A label the element names: the definition of a link reference or of a footnote.
    Label(LabelText<'a>),
    /// The anchors the raw HTML of the element gives, if it gives any.
    Html(HtmlText<'a>),
    /// Nothing.
    Nothing,
}
