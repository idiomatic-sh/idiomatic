//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no
//! ports. It maps a kind to the number the node table holds and back, says its name, and says the role a node of a kind has under its
//! parent.

use ryo::languages;

use super::constants::{
    ACTION, ANCHOR, AREA, AUDIO, BASE, BLOCKQUOTE, BUTTON, CITE, DATA, DEL, EMBED, EMPTY, FORM, FORMACTION, HREF, HTML_NAMESPACE,
    IFRAME, IMG, INLINE_QUOTE, INPUT, INS, LINK, OBJECT, POSTER, SCRIPT, SOURCE, SRC, SVG_NAMESPACE, TRACK, VIDEO, XLINK_NAMESPACE,
};
use super::types::{Field, Kind, Property, Reference};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use strum::IntoEnumIterator as _;

ryo::languages::kinds!(pub Kind { Node(Node), Token(Token), Extra(Extra) } unknown Kind::Extra(Extra::Error));

/// Whether the attribute `attribute` of the element `element`, each given as its namespace then its local name, holds a URL: the
/// attributes HTML defines as URLs, and an SVG `href`.
#[must_use]
pub(crate) fn url_valued(
    element: [languages::NameText<'_>; 2],
    attribute: [languages::NameText<'_>; 2],
) -> Reference {
    let [element_namespace, tag] = element.map(|each| each.0);
    let [namespace, name] = attribute.map(|each| each.0);
    let matches = match element_namespace {
        HTML_NAMESPACE if namespace.is_empty() => match name {
            HREF => matches!(tag, ANCHOR | AREA | BASE | LINK),
            SRC => matches!(
                tag,
                AUDIO | EMBED | IFRAME | IMG | INPUT | SCRIPT | SOURCE | TRACK | VIDEO
            ),
            ACTION => tag == FORM,
            POSTER => tag == VIDEO,
            CITE => matches!(tag, BLOCKQUOTE | DEL | INS | INLINE_QUOTE),
            DATA => tag == OBJECT,
            FORMACTION => matches!(tag, BUTTON | INPUT),
            _ => false,
        },
        SVG_NAMESPACE => name == HREF && matches!(namespace, EMPTY | XLINK_NAMESPACE),
        _ => false,
    };
    if matches {
        Reference::Url
    } else {
        Reference::Other
    }
}

/// The stable scalar-property number exposed by this language.
#[must_use]
pub(crate) const fn property_id(property: Property) -> languages::PropertyId {
    languages::PropertyId(match property {
        Property::Name => 1,
        Property::Namespace => 2,
        Property::Prefix => 3,
        Property::Value => 4,
        Property::PublicId => 5,
        Property::SystemId => 6,
        Property::DocumentMode => 7,
    })
}

/// The node a DOM scalar property is written as under the node it belongs to.
#[must_use]
pub(crate) const fn property_node(property: Property) -> Node {
    match property {
        Property::Name => Node::Name,
        Property::Namespace => Node::Namespace,
        Property::Prefix => Node::Prefix,
        Property::Value => Node::Value,
        Property::PublicId => Node::PublicId,
        Property::SystemId => Node::SystemId,
        Property::DocumentMode => Node::DocumentMode,
    }
}

/// The number the element table holds for a role.
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "a role is a field-less enum with a u16 representation: its number is the cast"
)]
pub(crate) const fn field_bits(field: Field) -> languages::FieldId {
    languages::FieldId(field as u16)
}

/// A role's name, in kebab case: text that leaves the engine.
#[must_use]
pub(crate) fn field_name(field: Field) -> languages::NameText<'static> {
    languages::NameText(field.into())
}

/// Every role, in the order of their numbers.
pub(crate) fn fields() -> impl Iterator<Item = Field> {
    Field::iter()
}

/// The role an element of `kind` has under its parent: the role is the kind's alone.
#[must_use]
pub(crate) fn field_of_kind(kind: Kind) -> Field {
    match kind {
        Kind::Node(Node::Name) => Field::Name,
        Kind::Node(Node::Namespace) => Field::Namespace,
        Kind::Node(Node::Prefix) => Field::Prefix,
        Kind::Node(Node::Value) => Field::Value,
        Kind::Node(Node::PublicId) => Field::PublicId,
        Kind::Node(Node::SystemId) => Field::SystemId,
        Kind::Node(Node::DocumentMode) => Field::DocumentMode,
        Kind::Node(Node::Attribute) => Field::Attribute,
        Kind::Node(
            Node::SourceFile
            | Node::Document
            | Node::DocumentType
            | Node::Element
            | Node::Text
            | Node::Comment
            | Node::DocumentFragment,
        )
        | Kind::Token(_)
        | Kind::Extra(_) => Field::None,
    }
}

