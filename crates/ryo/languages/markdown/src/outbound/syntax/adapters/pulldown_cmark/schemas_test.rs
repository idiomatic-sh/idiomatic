//! The schemas' cases: each of the library's kinds is the kind of the same meaning, and every kind the library names is mapped to a
//! kind of the language or named as dropped with its reason.

use super::super::constants::DEPTHS;
use super::super::types::Within;
use super::{depth_of, event_kind, tag_kind};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::MarkdownNode;
use crate::domain::syntax::service::{kind_bits, kind_of};
use crate::domain::syntax::types::Kind;
use pulldown_cmark::{
    BlockQuoteKind, CodeBlockKind, CowStr, Event, HeadingLevel, LinkType, MetadataBlockKind, Tag,
    TagEnd,
};

/// A kind of the library: the construct a tag opens, or a piece of the document the library gives as one event.
#[derive(Clone, Debug)]
enum Piece {
    /// A construct.
    Construct(Tag<'static>),
    /// A piece of the document, in the text of a document.
    Leaf(Event<'static>),
}

/// What the library gave: one kind of it, the depth of a heading, the text of a block of HTML, or all the kinds.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// One construct of the library, made by the function.
    One(fn() -> Tag<'static>),
    /// The depth the marker of a heading of this level carries.
    Depth(HeadingLevel),
    /// The kind of the text a block of HTML gives as an event.
    Indent,
    /// Every kind the library names.
    Every,
}

type Input = Given;
/// The kind of the language it is, the text of a depth, whether the indentation is dropped, or whether every kind of the library is
/// accounted for.
type Value = Seen;
type Failure = ();

/// What a case looks at.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    /// The kind of the language.
    Kind(Kind),
    /// The depth, as text.
    Depth(&'static str),
    /// Whether the kind is dropped.
    Dropped(bool),
    /// Whether every kind of the library is mapped to a kind of the language or dropped with a reason.
    Accounted(bool),
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_heading_is_the_kind_of_its_level"),
        word: Word::Succeeds,
        input: || {
            Given::One(|| Tag::Heading {
                level: HeadingLevel::H3,
                id: None,
                classes: Vec::new(),
                attrs: Vec::new(),
            })
        },
        expect: Expect::Value(Seen::Kind(Kind::Node(MarkdownNode::Heading))),
    },
    Case {
        name: CaseName("a_heading_carries_its_depth_as_text"),
        word: Word::Succeeds,
        input: || Given::Depth(HeadingLevel::H4),
        expect: Expect::Value(Seen::Depth(DEPTHS[3])),
    },
    Case {
        name: CaseName("a_quote_with_an_alert_is_the_alert"),
        word: Word::Succeeds,
        input: || Given::One(|| Tag::BlockQuote(Some(BlockQuoteKind::Warning))),
        expect: Expect::Value(Seen::Kind(Kind::Node(MarkdownNode::Blockquote))),
    },
    Case {
        name: CaseName("a_quote_without_one_is_a_quote"),
        word: Word::Succeeds,
        input: || Given::One(|| Tag::BlockQuote(None)),
        expect: Expect::Value(Seen::Kind(Kind::Node(MarkdownNode::Blockquote))),
    },
    Case {
        name: CaseName("a_fenced_block_is_one_whatever_its_info"),
        word: Word::Succeeds,
        input: || Given::One(|| Tag::CodeBlock(CodeBlockKind::Fenced(CowStr::Borrowed("rust")))),
        expect: Expect::Value(Seen::Kind(Kind::Node(MarkdownNode::Code))),
    },
    Case {
        name: CaseName("a_numbered_list_is_ordered"),
        word: Word::Succeeds,
        input: || Given::One(|| Tag::List(Some(1))),
        expect: Expect::Value(Seen::Kind(Kind::Node(MarkdownNode::List))),
    },
    Case {
        name: CaseName("front_matter_between_pluses_is_toml"),
        word: Word::Succeeds,
        input: || Given::One(|| Tag::MetadataBlock(MetadataBlockKind::PlusesStyle)),
        expect: Expect::Value(Seen::Kind(Kind::Extra(Extra::Toml))),
    },
    Case {
        name: CaseName("the_indentation_of_a_block_of_html_is_dropped"),
        word: Word::Succeeds,
        input: || Given::Indent,
        expect: Expect::Value(Seen::Dropped(true)),
    },
    Case {
        name: CaseName("every_kind_of_the_library_is_mapped_or_dropped"),
        word: Word::Succeeds,
        input: || Given::Every,
        expect: Expect::Value(Seen::Accounted(true)),
    },
];

/// A link or an image of the library written as `link_type` says.
fn linked(link_type: LinkType) -> [Tag<'static>; 2] {
    [
        Tag::Link {
            link_type,
            dest_url: CowStr::Borrowed(""),
            title: CowStr::Borrowed(""),
            id: CowStr::Borrowed(""),
        },
        Tag::Image {
            link_type,
            dest_url: CowStr::Borrowed(""),
            title: CowStr::Borrowed(""),
            id: CowStr::Borrowed(""),
        },
    ]
}

/// Every kind the library names, from the library's own enumeration: one entry for each tag and each event, and each way a link is
/// written, so that a kind the library adds is tested the day it is added and not when a file first meets it.
fn library_kinds() -> Vec<Piece> {
    let mut constructs = vec![
        Tag::Paragraph,
        Tag::Heading {
            level: HeadingLevel::H1,
            id: None,
            classes: Vec::new(),
            attrs: Vec::new(),
        },
        Tag::BlockQuote(None),
        Tag::CodeBlock(CodeBlockKind::Indented),
        Tag::HtmlBlock,
        Tag::List(None),
        Tag::Item,
        Tag::FootnoteDefinition(CowStr::Borrowed("")),
        Tag::DefinitionList,
        Tag::DefinitionListTitle,
        Tag::DefinitionListDefinition,
        Tag::Table(Vec::new()),
        Tag::TableHead,
        Tag::TableRow,
        Tag::TableCell,
        Tag::Emphasis,
        Tag::Strong,
        Tag::Strikethrough,
        Tag::Superscript,
        Tag::Subscript,
        Tag::MetadataBlock(MetadataBlockKind::YamlStyle),
        Tag::MetadataBlock(MetadataBlockKind::PlusesStyle),
    ];
    for link_type in [
        LinkType::Inline,
        LinkType::Reference,
        LinkType::ReferenceUnknown,
        LinkType::Collapsed,
        LinkType::CollapsedUnknown,
        LinkType::Shortcut,
        LinkType::ShortcutUnknown,
        LinkType::Autolink,
        LinkType::Email,
        LinkType::WikiLink { has_pothole: false },
    ] {
        constructs.extend(linked(link_type));
    }
    let leaves = [
        Event::Start(Tag::Paragraph),
        Event::End(TagEnd::Paragraph),
        Event::Text(CowStr::Borrowed("")),
        Event::Code(CowStr::Borrowed("")),
        Event::InlineMath(CowStr::Borrowed("")),
        Event::DisplayMath(CowStr::Borrowed("")),
        Event::Html(CowStr::Borrowed("")),
        Event::InlineHtml(CowStr::Borrowed("")),
        Event::FootnoteReference(CowStr::Borrowed("")),
        Event::SoftBreak,
        Event::HardBreak,
        Event::Rule,
        Event::TaskListMarker(true),
        Event::TaskListMarker(false),
    ];
    constructs
        .into_iter()
        .map(Piece::Construct)
        .chain(leaves.into_iter().map(Piece::Leaf))
        .collect()
}

/// Why the language has no kind for a kind of the library, or none when it has one. The match names every tag and every event of the
/// library, so that a kind the library adds is a compile error here until it is mapped or named as dropped.
fn dropped(piece: &Piece) -> Option<&'static str> {
    match piece {
        Piece::Construct(
            Tag::Paragraph
            | Tag::Heading { .. }
            | Tag::BlockQuote(_)
            | Tag::CodeBlock(_)
            | Tag::HtmlBlock
            | Tag::List(_)
            | Tag::Item
            | Tag::FootnoteDefinition(_)
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell
            | Tag::Emphasis
            | Tag::Strong
            | Tag::Strikethrough
            | Tag::Superscript
            | Tag::Subscript
            | Tag::Link { .. }
            | Tag::Image { .. }
            | Tag::MetadataBlock(_),
        )
        | Piece::Leaf(
            Event::Text(_)
            | Event::Code(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::Html(_)
            | Event::InlineHtml(_)
            | Event::FootnoteReference(_)
            | Event::SoftBreak
            | Event::HardBreak
            | Event::Rule
            | Event::TaskListMarker(_),
        ) => None,
        Piece::Leaf(Event::Start(_) | Event::End(_)) => {
            Some("the events that open and close a construct are the construct, named by its tag")
        }
    }
}

/// Whether the language answers for `piece`: it is dropped with a reason, or it is mapped to a kind the language has.
fn accounted(piece: &Piece) -> bool {
    if let Some(reason) = dropped(piece) {
        return !reason.is_empty();
    }
    let kind = match piece {
        Piece::Construct(tag) => Some(tag_kind(tag)),
        Piece::Leaf(event) => event_kind(event, Within::Prose),
    };
    kind.is_some_and(|each| kind_of(kind_bits(each)) == each)
}

fn subject(given: Input) -> Value {
    match given {
        Given::One(tag) => Seen::Kind(tag_kind(&tag())),
        Given::Depth(level) => Seen::Depth(depth_of(level)),
        Given::Indent => {
            Seen::Dropped(event_kind(&Event::Text(CowStr::Borrowed(" ")), Within::Indent).is_none())
        }
        Given::Every => Seen::Accounted(library_kinds().iter().all(accounted)),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
