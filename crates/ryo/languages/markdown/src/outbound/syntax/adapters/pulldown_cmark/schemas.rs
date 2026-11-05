//! What the parser gives, which the adapter does not control, and its conversion into the language's kinds: the one declaration of
//! how each of the library's tags is a kind of the language's trees.

use super::constants::DEPTHS;
use super::types::Within;
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use pulldown_cmark::{Event, HeadingLevel, LinkType, MetadataBlockKind, Tag};

/// A tag the library opens a construct with.
pub(super) struct TagSchema<'t, 'a>(pub &'t Tag<'a>);

/// Whether a link or an image is written with its destination, or with a label that names a definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WrittenSchema {
    /// With the destination: `[a](b)`, `<b>`.
    Destination,
    /// With a label: `[a][b]`, `[a][]`, `[a]`.
    Label,
}

/// How the library says a link is written.
fn written_as(link: LinkType) -> WrittenSchema {
    match link {
        LinkType::Inline | LinkType::Autolink | LinkType::Email | LinkType::WikiLink { .. } => {
            WrittenSchema::Destination
        }
        LinkType::Reference
        | LinkType::ReferenceUnknown
        | LinkType::Collapsed
        | LinkType::CollapsedUnknown
        | LinkType::Shortcut
        | LinkType::ShortcutUnknown => WrittenSchema::Label,
    }
}

impl From<TagSchema<'_, '_>> for Kind {
    fn from(schema: TagSchema<'_, '_>) -> Self {
        match schema.0 {
            Tag::Paragraph => Self::Node(Node::Paragraph),
            Tag::Heading { .. } => Self::Node(Node::Heading),
            Tag::BlockQuote(_) => Self::Node(Node::Blockquote),
            Tag::CodeBlock(_) => Self::Node(Node::Code),
            Tag::HtmlBlock => Self::Node(Node::Html),
            Tag::List(_) => Self::Node(Node::List),
            Tag::Item => Self::Node(Node::ListItem),
            Tag::FootnoteDefinition(_) => Self::Node(Node::FootnoteDefinition),
            Tag::DefinitionList => Self::Extra(Extra::DefinitionList),
            Tag::DefinitionListTitle => Self::Extra(Extra::DefinitionListTitle),
            Tag::DefinitionListDefinition => Self::Extra(Extra::DefinitionListDefinition),
            Tag::Table(_) => Self::Node(Node::Table),
            Tag::TableHead | Tag::TableRow => Self::Node(Node::TableRow),
            Tag::TableCell => Self::Node(Node::TableCell),
            Tag::Emphasis => Self::Node(Node::Emphasis),
            Tag::Strong => Self::Node(Node::Strong),
            Tag::Strikethrough => Self::Node(Node::Delete),
            Tag::Superscript => Self::Extra(Extra::Superscript),
            Tag::Subscript => Self::Extra(Extra::Subscript),
            Tag::Link { link_type, .. } => match written_as(*link_type) {
                WrittenSchema::Destination => Self::Node(Node::Link),
                WrittenSchema::Label => Self::Node(Node::LinkReference),
            },
            Tag::Image { link_type, .. } => match written_as(*link_type) {
                WrittenSchema::Destination => Self::Node(Node::Image),
                WrittenSchema::Label => Self::Node(Node::ImageReference),
            },
            Tag::MetadataBlock(MetadataBlockKind::YamlStyle) => Self::Extra(Extra::Yaml),
            Tag::MetadataBlock(MetadataBlockKind::PlusesStyle) => Self::Extra(Extra::Toml),
        }
    }
}

/// How deep the library says a heading is, as the text the adapter carries on its marker.
pub(super) fn depth_of(level: HeadingLevel) -> &'static str {
    match level {
        HeadingLevel::H1 => DEPTHS[0],
        HeadingLevel::H2 => DEPTHS[1],
        HeadingLevel::H3 => DEPTHS[2],
        HeadingLevel::H4 => DEPTHS[3],
        HeadingLevel::H5 => DEPTHS[4],
        HeadingLevel::H6 => DEPTHS[5],
    }
}

/// How deep a heading of `level` is, from 1.
pub(super) const fn level_of(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// The language's kind for a construct the library opens with `tag`.
pub(super) fn tag_kind(tag: &Tag<'_>) -> Kind {
    Kind::from(TagSchema(tag))
}

/// The language's kind for a piece of the document the library gives as one event, in the text `within`; nothing for the events that
/// open or close a construct, which `tag_kind` names, and for the indentation of a block of HTML, whose bytes are white space.
pub(super) fn event_kind(event: &Event<'_>, within: Within) -> Option<Kind> {
    match event {
        Event::Start(_) | Event::End(_) => None,
        Event::Text(_) => match within {
            Within::Code => Some(Kind::Token(Token::CodeFlowValue)),
            Within::Indent => None,
            Within::Prose => Some(Kind::Node(Node::Text)),
        },
        Event::Code(_) => Some(Kind::Node(Node::InlineCode)),
        Event::InlineMath(_) => Some(Kind::Extra(Extra::InlineMath)),
        Event::DisplayMath(_) => Some(Kind::Extra(Extra::Math)),
        Event::Html(_) => Some(Kind::Token(Token::HtmlFlowData)),
        Event::InlineHtml(_) => Some(Kind::Node(Node::Html)),
        Event::FootnoteReference(_) => Some(Kind::Node(Node::FootnoteReference)),
        Event::SoftBreak => Some(Kind::Token(Token::LineEnding)),
        Event::HardBreak => Some(Kind::Node(Node::Break)),
        Event::Rule => Some(Kind::Node(Node::ThematicBreak)),
        Event::TaskListMarker(true) => Some(Kind::Extra(Extra::TaskChecked)),
        Event::TaskListMarker(false) => Some(Kind::Extra(Extra::TaskUnchecked)),
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
