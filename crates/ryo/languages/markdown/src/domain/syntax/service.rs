//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no
//! ports. It maps a kind to the number the node table holds and back, says its name, and says the role a node of a kind has under
//! its parent.

use ryo::languages;

use super::constants::{COMMENT_CLOSE, COMMENT_OPEN};
use super::types::{Field, Kind, Level};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use strum::IntoEnumIterator as _;

ryo::languages::kinds!(pub Kind { Node(Node), Token(Token), Extra(Extra) } unknown Kind::Extra(Extra::Error));

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

/// The role of an element of `element` kind under a parent of `parent` kind, by the property of mdast's node that holds it.
#[must_use]
pub(crate) fn field_between(parent: Kind, element: Kind) -> Field {
    match (parent, element) {
        (Kind::Node(Node::Code), Kind::Token(Token::CodeFlowValue))
        | (Kind::Node(Node::Html), Kind::Token(Token::HtmlFlowData)) => Field::Value,
        (Kind::Node(Node::ListItem), Kind::Extra(Extra::TaskChecked | Extra::TaskUnchecked)) => {
            Field::Checked
        }
        (
            _,
            Kind::Node(_)
            | Kind::Extra(
                Extra::Yaml
                | Extra::Toml
                | Extra::Math
                | Extra::InlineMath
                | Extra::DefinitionList
                | Extra::DefinitionListTitle
                | Extra::DefinitionListDefinition
                | Extra::Superscript
                | Extra::Subscript,
            ),
        ) => Field::Children,
        (
            _,
            Kind::Token(_)
            | Kind::Extra(
                Extra::Markup
                | Extra::TaskChecked
                | Extra::TaskUnchecked
                | Extra::Error
                | Extra::Section,
            ),
        ) => Field::None,
    }
}
