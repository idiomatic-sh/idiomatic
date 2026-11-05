//! The types of the `nodes` concept: plain data, public fields and derives, with no constructor and no inherent impl. A node is named
//! as the standard names it, in kebab case.

/// What a node of a html syntax tree is. One closed list, each name once.
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
pub(crate) enum HtmlNode {
    /// The HTML `SourceFile` field or node.
    SourceFile,
    /// The native HTML document.
    Document,
    /// The HTML `DocumentType` field or node.
    DocumentType,
    /// The HTML Element field or node.
    Element,
    /// The HTML Attribute field or node.
    Attribute,
    /// The HTML Text field or node.
    Text,
    /// The HTML Comment field or node.
    Comment,
    /// The HTML `DocumentFragment` field or node.
    DocumentFragment,
    /// The local name of an element, an attribute or a doctype, as a node of its own under it: its text is the name.
    Name,
    /// The namespace URI of an element or an attribute, as a node of its own under it.
    Namespace,
    /// The namespace prefix of a foreign attribute (`xlink` of `xlink:href`), as a node of its own under it.
    Prefix,
    /// The decoded value of an attribute, a text or a comment, as a node of its own under it.
    Value,
    /// The public identifier of a doctype, as a node of its own under it.
    PublicId,
    /// The system identifier of a doctype, as a node of its own under it.
    SystemId,
    /// The quirks mode of the document, as a node of its own under it.
    DocumentMode,
}
