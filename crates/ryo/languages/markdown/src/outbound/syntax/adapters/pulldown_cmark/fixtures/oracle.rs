//! The library's own reading of a file, its events, compared with the rows: every heading (with its level), link, image, code block
//! and emphasis the library opens is a `syntax_tree` node of the kind and the span of its event, and no other node of those kinds is;
//! and the code of every code block and code span, as the library gives it, is the `string_literal` of the same span. What the oracle
//! rail of the conformance holds `syntax_tree` and `string_literal` to.

use crate::domain::nodes::types::Node;
use pulldown_cmark::{Event, LinkType, Parser, Tag, TagEnd};
use ryo::languages::Relation;
use ryo::languages::contract::{Cell, OracleRows, Point, point_of};

use super::super::constants::OPTIONS;
use crate::domain::syntax::service::kind_name;
use crate::domain::syntax::types::Kind;

/// The kinds of construct the oracle compares.
const COMPARED: [Kind; 13] = [
    Kind::Node(Node::Heading),
    Kind::Node(Node::Heading),
    Kind::Node(Node::Heading),
    Kind::Node(Node::Heading),
    Kind::Node(Node::Heading),
    Kind::Node(Node::Heading),
    Kind::Node(Node::Link),
    Kind::Node(Node::Image),
    Kind::Node(Node::Code),
    Kind::Node(Node::Code),
    Kind::Node(Node::Emphasis),
    Kind::Node(Node::Strong),
    Kind::Node(Node::Delete),
];

/// The kind of the compared constructs the library opens with `tag`: the oracle's own table, which names no other construct.
fn compared(tag: &Tag<'_>) -> Option<Kind> {
    match tag {
        Tag::Heading { .. } => Some(Kind::Node(Node::Heading)),
        Tag::Link { link_type, .. } if written_with_destination(*link_type) => {
            Some(Kind::Node(Node::Link))
        }
        Tag::Image { link_type, .. } if written_with_destination(*link_type) => {
            Some(Kind::Node(Node::Image))
        }
        Tag::CodeBlock(_) => Some(Kind::Node(Node::Code)),
        Tag::Emphasis => Some(Kind::Node(Node::Emphasis)),
        Tag::Strong => Some(Kind::Node(Node::Strong)),
        Tag::Strikethrough => Some(Kind::Node(Node::Delete)),
        Tag::Link { .. }
        | Tag::Image { .. }
        | Tag::Paragraph
        | Tag::BlockQuote(_)
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
        | Tag::Superscript
        | Tag::Subscript
        | Tag::MetadataBlock(_) => None,
    }
}

/// Whether the library says a link is written with its destination, and not with a label.
fn written_with_destination(link: LinkType) -> bool {
    matches!(
        link,
        LinkType::Inline | LinkType::Autolink | LinkType::Email | LinkType::WikiLink { .. }
    )
}

/// A construct or a piece of code, as both sides say it: its kind or its code, and where it starts and ends.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Said {
    /// The kind of the construct, or the code.
    what: String,
    /// Where it starts.
    from: Point,
    /// Where it ends.
    to: Point,
}

/// What one side says of a file: the constructs of the compared kinds, and the code.
struct Reading {
    /// The constructs.
    constructs: Vec<Said>,
    /// The code.
    code: Vec<Said>,
}

/// The span of a cell, as a stretch of `what`; nothing for any other cell.
fn said(what: String, cell: Option<&Cell>) -> Option<Said> {
    match cell {
        Some(Cell::Span(from, to)) => Some(Said {
            what,
            from: *from,
            to: *to,
        }),
        _ => None,
    }
}

/// The text of a name cell; nothing for any other.
fn name(cell: Option<&Cell>) -> Option<String> {
    match cell {
        Some(Cell::Name(text)) => Some(text.clone()),
        _ => None,
    }
}

/// What the library's events say of `text`: the constructs of the compared kinds, and the code.
fn library(text: &str) -> Reading {
    let bytes = text.as_bytes();
    let at = |offset| point_of(bytes, u32::try_from(offset).unwrap_or(u32::MAX));
    let stretch = |what, start, end| Said {
        what,
        from: at(start),
        to: at(end),
    };
    let (mut constructs, mut code) = (Vec::new(), Vec::new());
    let (mut lines, mut opened) = (String::new(), None);
    for (event, range) in Parser::new_ext(text, OPTIONS).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if let Some(kind) = compared(&tag) {
                    constructs.push(stretch(
                        kind_name(kind).0.to_owned(),
                        range.start,
                        range.end,
                    ));
                }
                if matches!(tag, Tag::CodeBlock(_)) {
                    lines.clear();
                    opened = Some(range.clone());
                }
            }
            Event::Text(said) => {
                if opened.is_some() {
                    lines.push_str(&said);
                }
            }
            Event::Code(said) => code.push(stretch(said.to_string(), range.start, range.end)),
            Event::End(TagEnd::CodeBlock) => {
                if let Some(block) = opened.take() {
                    code.push(stretch(std::mem::take(&mut lines), block.start, block.end));
                }
            }
            Event::End(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::Html(_)
            | Event::InlineHtml(_)
            | Event::FootnoteReference(_)
            | Event::SoftBreak
            | Event::HardBreak
            | Event::Rule
            | Event::TaskListMarker(_) => {}
        }
    }
    Reading { constructs, code }
}

/// What the rows say: the `syntax_tree` nodes of the compared kinds, and the `string_literal` rows.
fn rows_say(rows: &OracleRows) -> Reading {
    let compared: Vec<&str> = COMPARED.iter().map(|kind| kind_name(*kind).0).collect();
    let constructs = rows
        .of(Relation::SyntaxTree)
        .iter()
        .filter_map(|row| {
            let kind = name(row.get(1))?;
            if compared.contains(&kind.as_str()) {
                said(kind, row.get(4))
            } else {
                None
            }
        })
        .collect();
    let code = rows
        .of(Relation::StringLiteral)
        .iter()
        .filter_map(|row| said(name(row.get(2))?, row.get(4)))
        .collect();
    Reading { constructs, code }
}

/// What differs between two lists, at most a few sentences.
fn differences(what: &str, mut ours: Vec<Said>, mut theirs: Vec<Said>) -> Vec<String> {
    ours.sort();
    theirs.sort();
    let mut found: Vec<String> = theirs
        .iter()
        .filter(|each| !ours.contains(each))
        .map(|each| {
            format!(
                "the library has the {what} {:?} at {}:{}-{}:{} and the rows do not",
                each.what, each.from.line, each.from.column, each.to.line, each.to.column
            )
        })
        .collect();
    found.extend(
        ours.iter()
            .filter(|each| !theirs.contains(each))
            .map(|each| {
                format!(
                    "the rows have the {what} {:?} at {}:{}-{}:{} and the library does not",
                    each.what, each.from.line, each.from.column, each.to.line, each.to.column
                )
            }),
    );
    found.truncate(5);
    found
}

/// Compares the headings, links, images, code blocks, emphasis and code the rows of `text` say with the events the library gives for
/// it; what differs, one sentence each; nothing when they agree.
pub(crate) fn oracle(text: &str, rows: &OracleRows) -> Vec<String> {
    let theirs = library(text);
    let ours = rows_say(rows);
    let mut found = differences("construct", ours.constructs, theirs.constructs);
    found.extend(differences("code", ours.code, theirs.code));
    found
}
