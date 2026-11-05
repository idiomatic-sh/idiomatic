//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no
//! ports. It maps a kind to the number the element table holds and back, says its name, and reads the kind of an element of a file's
//! syntax tree: a position is an element of the table; its kind is the language's own.

use ryo::languages::{self, Tree};

use super::constants::{
    ACTION, ANCHOR, AREA, ARTICLE, ASIDE, AUDIO, BASE, BLOCKQUOTE, BUTTON, CITE, DATA, DEL, EMBED,
    EMPTY, FORM, FORMACTION, HEADINGS, HREF, HTML_NAMESPACE, ID, IFRAME, IMG, INLINE_QUOTE, INPUT,
    INS, LINK, NAME, NAV, OBJECT, POSTER, SCRIPT, SECTION, SOURCE, SRC, SVG_NAMESPACE,
    TARGET_PROPERTY, TRACK, VIDEO, WORD_SPACE, XLINK_NAMESPACE,
};
use super::types::{Anchor, Defined, Field, Kind, Property, Reference, Rootness, Section, Wording};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::HtmlNode;
use crate::domain::tokens::types::Token;
use strum::IntoEnumIterator as _;

ryo::languages::kinds!(pub Kind { Node(HtmlNode), Token(Token), Extra(Extra) } unknown Kind::Extra(Extra::Error));

/// The kind of the element at `at`.
#[must_use]
pub(crate) fn kind(tree: &Tree<'_>, at: languages::Position) -> Kind {
    kind_of(languages::kind_at(tree.elements, at))
}

/// The name of the kind of the element at `at`, as the `kind` column of the tree holds it.
pub(crate) fn kind_symbol(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    let element = kind(tree, at);
    languages::vocabulary_symbol(symbols, tree.source, kind_bits(element), kind_name(element))
}

/// The text of the token at `at`; no text for a node.
#[must_use]
pub(crate) fn token_text(tree: &Tree<'_>, at: languages::Position) -> languages::Symbol {
    languages::text_at(tree.elements, at)
}

/// The parent of the element at `at`; the root is its own.
#[must_use]
pub(crate) fn parent(tree: &Tree<'_>, at: languages::Position) -> languages::Position {
    languages::parent_at(tree.elements, at)
}

/// Whether `at` is the root of the tree.
#[must_use]
pub(crate) fn is_root(tree: &Tree<'_>, at: languages::Position) -> Rootness {
    if parent(tree, at) == at {
        Rootness::Root
    } else {
        Rootness::Inner
    }
}

/// The place of the element at `at` among its parent's children.
#[must_use]
pub(crate) fn ordinal(tree: &Tree<'_>, at: languages::Position) -> languages::Integer {
    languages::Integer(i64::from(languages::ordinal_at(tree.elements, at).0))
}

/// The place of the `number`-th thing the parser reported about the file, as a node id of the file: a report is known by its number.
#[must_use]
pub(crate) fn diagnostic_id(tree: &Tree<'_>, number: languages::ChildOrdinal) -> languages::NodeId {
    languages::diagnostic_id(tree.file, number)
}

/// What the parser said at one place, as a name.
pub(crate) fn diagnostic_message(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    diagnostic: &languages::Diagnostic<'_>,
) -> languages::Symbol {
    languages::intern_derived(
        symbols,
        tree.source,
        languages::NameText(std::str::from_utf8(diagnostic.message.0).unwrap_or_default()),
    )
}

/// Where the parser said it.
#[must_use]
pub(crate) fn diagnostic_span(diagnostic: &languages::Diagnostic<'_>) -> languages::Span {
    languages::Span {
        start: diagnostic.start,
        end: diagnostic.end,
    }
}

/// A decoded DOM field under this node, never a reparse of source markup.
#[must_use]
pub(crate) fn field<'a>(
    tree: &Tree<'a>,
    at: languages::Position,
    field: Property,
) -> languages::NameText<'a> {
    languages::property_at(tree.elements, at, property_id(field))
        .unwrap_or(languages::NameText(EMPTY))
}
/// The decoded value attached to a DOM node.
pub(crate) fn value(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    languages::intern_derived(symbols, tree.source, field(tree, at, Property::Value))
}
/// Whether an element starts a document section.
#[must_use]
pub(crate) fn section(tree: &Tree<'_>, at: languages::Position) -> Section {
    let matches = || {
        kind(tree, at) == Kind::Node(HtmlNode::Element)
            && matches!(
                field(tree, at, Property::Name).0,
                ARTICLE | ASIDE | NAV | SECTION
            )
    };
    if matches() {
        Section::Section
    } else {
        Section::Other
    }
}
/// The closest section containing a node, or the document.
#[must_use]
pub(crate) fn owner(tree: &Tree<'_>, at: languages::Position) -> languages::Position {
    let mut cursor = parent(tree, at);
    while cursor != parent(tree, cursor) && section(tree, cursor) != Section::Section {
        cursor = parent(tree, cursor);
    }
    cursor
}
/// An HTML identifier attribute defines an anchor.
#[must_use]
pub(crate) fn anchor(tree: &Tree<'_>, at: languages::Position) -> Anchor {
    let matches = || {
        if !field(tree, at, Property::Namespace).0.is_empty()
            || field(tree, at, Property::Value).0.is_empty()
        {
            return false;
        }
        field(tree, at, Property::Name).0 == ID
            || (field(tree, at, Property::Name).0 == NAME
                && field(tree, parent(tree, at), Property::Name).0 == ANCHOR
                && field(tree, parent(tree, at), Property::Namespace).0 == HTML_NAMESPACE)
    };
    if matches() {
        Anchor::Definition
    } else {
        Anchor::Other
    }
}
/// What the element or attribute at `at` defines: a sectioning element whose title is a heading is a section, an anchor is a label.
#[must_use]
pub(crate) fn defined(tree: &Tree<'_>, at: languages::Position) -> Defined {
    if anchor(tree, at) == Anchor::Definition {
        return Defined::Label;
    }
    if section(tree, at) == Section::Section
        && let Some(heading) = title(tree, at)
    {
        return Defined::Section(heading);
    }
    Defined::Nothing
}

/// The heading that titles the section at `at`: the first `h1` to `h6` it holds, outside the sections it holds.
#[must_use]
fn title(tree: &Tree<'_>, at: languages::Position) -> Option<languages::Position> {
    languages::children(tree.elements, at).find_map(|child| {
        if kind(tree, child) != Kind::Node(HtmlNode::Element)
            || section(tree, child) == Section::Section
        {
            return None;
        }
        if HEADINGS.contains(&field(tree, child, Property::Name).0) {
            return Some(child);
        }
        title(tree, child)
    })
}

/// The title of a section, as a name: the words of the text the heading at `heading` holds, one space apart.
pub(crate) fn section_name(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    heading: languages::Position,
) -> languages::Symbol {
    let from = languages::begin_name(symbols);
    let mut any = Wording::Silent;
    words(tree, symbols, heading, &mut any);
    languages::end_name(symbols, tree.source, from)
}

/// Writes the words of the text the element at `at` holds into the name being made, one space apart.
fn words(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
    any: &mut Wording,
) {
    for child in languages::children(tree.elements, at) {
        match kind(tree, child) {
            Kind::Node(HtmlNode::Text) => {
                for word in field(tree, child, Property::Value).0.split_whitespace() {
                    if *any == Wording::Words {
                        languages::push_text(symbols, WORD_SPACE);
                    }
                    languages::push_text(symbols, languages::NameText(word));
                    *any = Wording::Words;
                }
            }
            Kind::Node(HtmlNode::Element) => words(tree, symbols, child, any),
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => {}
        }
    }
}

/// URL-valued attributes are references; event handlers are not HTML code relations.
#[must_use]
pub(crate) fn reference(tree: &Tree<'_>, at: languages::Position) -> Reference {
    let element = parent(tree, at);
    url_valued(
        [
            field(tree, element, Property::Namespace),
            field(tree, element, Property::Name),
        ],
        [
            field(tree, at, Property::Namespace),
            field(tree, at, Property::Name),
        ],
    )
}

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

/// The target the URL-valued attribute at `at` points at, resolved to a path of the run as the adapter wrote it: as a name of the
/// file.
pub(crate) fn target(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    languages::intern_derived(
        symbols,
        tree.source,
        languages::property_at(tree.elements, at, TARGET_PROPERTY)
            .unwrap_or(languages::NameText(EMPTY)),
    )
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
pub(crate) const fn property_node(property: Property) -> HtmlNode {
    match property {
        Property::Name => HtmlNode::Name,
        Property::Namespace => HtmlNode::Namespace,
        Property::Prefix => HtmlNode::Prefix,
        Property::Value => HtmlNode::Value,
        Property::PublicId => HtmlNode::PublicId,
        Property::SystemId => HtmlNode::SystemId,
        Property::DocumentMode => HtmlNode::DocumentMode,
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

/// The role of the element at `at` under its parent: a property's node is in the role of its property, an attribute in the role
/// `attribute` under its element, and every other element has none.
#[must_use]
pub(crate) fn field_of(tree: &Tree<'_>, at: languages::Position) -> Field {
    match kind(tree, at) {
        Kind::Node(HtmlNode::Name) => Field::Name,
        Kind::Node(HtmlNode::Namespace) => Field::Namespace,
        Kind::Node(HtmlNode::Prefix) => Field::Prefix,
        Kind::Node(HtmlNode::Value) => Field::Value,
        Kind::Node(HtmlNode::PublicId) => Field::PublicId,
        Kind::Node(HtmlNode::SystemId) => Field::SystemId,
        Kind::Node(HtmlNode::DocumentMode) => Field::DocumentMode,
        Kind::Node(HtmlNode::Attribute) => Field::Attribute,
        Kind::Node(
            HtmlNode::SourceFile
            | HtmlNode::Document
            | HtmlNode::DocumentType
            | HtmlNode::Element
            | HtmlNode::Text
            | HtmlNode::Comment
            | HtmlNode::DocumentFragment,
        )
        | Kind::Token(_)
        | Kind::Extra(_) => Field::None,
    }
}

/// The role of the element at `at`, as the number the element table holds.
#[must_use]
pub(crate) fn role_of(tree: &Tree<'_>, at: languages::Position) -> languages::FieldId {
    field_bits(field_of(tree, at))
}
