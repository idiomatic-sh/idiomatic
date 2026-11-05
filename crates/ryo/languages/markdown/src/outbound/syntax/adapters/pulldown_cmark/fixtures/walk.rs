//! The library's own tree of a file, walked in the language's kinds: what the shape rail of the conformance holds our tree to.
//! `pulldown-cmark` gives no tree, only a stream of events with their spans, so its tree is the one the events open and close: each
//! construct a node at the bytes its events span, each other event a token in the construct it comes in. The bytes no event spans
//! are then cut at every edge of a node or token and each stretch put, as white space or markup, in the deepest node whose bytes hold
//! it. A heading opens a section (the events give none) holding it and what follows up to a heading as deep or shallower. The adapter
//! writes the table in one pass over the stream; this places each stretch by its bytes alone, so a misplaced token is a difference.

use pulldown_cmark::{Event, HeadingLevel, LinkType, MetadataBlockKind, Parser, Tag};
use ryo::languages::Offset;
use ryo::languages::contract::{Leafness, Walked};

use super::super::constants::{OPTIONS, WHITE_SPACE};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::MarkdownNode;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// The offset of a byte index of the text.
fn offset(index: usize) -> Offset {
    Offset(u32::try_from(index).unwrap_or(u32::MAX))
}

/// A piece of the tree of kind `kind` over `start..end`.
fn piece(kind: Kind, leaf: Leafness, start: usize, end: usize) -> Walked {
    Walked {
        kind: kind_bits(kind),
        leaf,
        start: offset(start),
        end: offset(end),
        children: Vec::new(),
    }
}

/// Whether the library says a link is written with a label, `[a][b]`, `[a][]` or `[a]`, and not with its destination.
fn labelled(link: LinkType) -> bool {
    matches!(
        link,
        LinkType::Reference
            | LinkType::ReferenceUnknown
            | LinkType::Collapsed
            | LinkType::CollapsedUnknown
            | LinkType::Shortcut
            | LinkType::ShortcutUnknown
    )
}

/// The kind of the construct the library opens with `tag`, named by the walk's own table.
fn construct(tag: &Tag<'_>) -> Kind {
    match tag {
        Tag::Paragraph => Kind::Node(MarkdownNode::Paragraph),
        Tag::Heading { .. } => Kind::Node(MarkdownNode::Heading),
        Tag::BlockQuote(_) => Kind::Node(MarkdownNode::Blockquote),
        Tag::CodeBlock(_) => Kind::Node(MarkdownNode::Code),
        Tag::HtmlBlock => Kind::Node(MarkdownNode::Html),
        Tag::List(_) => Kind::Node(MarkdownNode::List),
        Tag::Item => Kind::Node(MarkdownNode::ListItem),
        Tag::FootnoteDefinition(_) => Kind::Node(MarkdownNode::FootnoteDefinition),
        Tag::DefinitionList => Kind::Extra(Extra::DefinitionList),
        Tag::DefinitionListTitle => Kind::Extra(Extra::DefinitionListTitle),
        Tag::DefinitionListDefinition => Kind::Extra(Extra::DefinitionListDefinition),
        Tag::Table(_) => Kind::Node(MarkdownNode::Table),
        Tag::TableHead | Tag::TableRow => Kind::Node(MarkdownNode::TableRow),
        Tag::TableCell => Kind::Node(MarkdownNode::TableCell),
        Tag::Emphasis => Kind::Node(MarkdownNode::Emphasis),
        Tag::Strong => Kind::Node(MarkdownNode::Strong),
        Tag::Strikethrough => Kind::Node(MarkdownNode::Delete),
        Tag::Superscript => Kind::Extra(Extra::Superscript),
        Tag::Subscript => Kind::Extra(Extra::Subscript),
        Tag::Link { link_type, .. } if labelled(*link_type) => Kind::Node(MarkdownNode::LinkReference),
        Tag::Link { .. } => Kind::Node(MarkdownNode::Link),
        Tag::Image { link_type, .. } if labelled(*link_type) => Kind::Node(MarkdownNode::ImageReference),
        Tag::Image { .. } => Kind::Node(MarkdownNode::Image),
        Tag::MetadataBlock(MetadataBlockKind::YamlStyle) => Kind::Extra(Extra::Yaml),
        Tag::MetadataBlock(MetadataBlockKind::PlusesStyle) => Kind::Extra(Extra::Toml),
    }
}

/// The kind of a token for an event that is not the start or end of a construct, in the construct `within`.
fn leaf_kind(event: &Event<'_>, within: Kind) -> Option<Kind> {
    Some(match event {
        Event::Text(_) if within == Kind::Node(MarkdownNode::Html) => return None,
        Event::Text(_) if within == Kind::Node(MarkdownNode::Code) => Kind::Token(Token::CodeFlowValue),
        Event::Text(_) => Kind::Node(MarkdownNode::Text),
        Event::Code(_) => Kind::Node(MarkdownNode::InlineCode),
        Event::InlineMath(_) => Kind::Extra(Extra::InlineMath),
        Event::DisplayMath(_) => Kind::Extra(Extra::Math),
        Event::Html(_) => Kind::Token(Token::HtmlFlowData),
        Event::InlineHtml(_) => Kind::Node(MarkdownNode::Html),
        Event::FootnoteReference(_) => Kind::Node(MarkdownNode::FootnoteReference),
        Event::SoftBreak => Kind::Token(Token::LineEnding),
        Event::HardBreak => Kind::Node(MarkdownNode::Break),
        Event::Rule => Kind::Node(MarkdownNode::ThematicBreak),
        Event::TaskListMarker(true) => Kind::Extra(Extra::TaskChecked),
        Event::TaskListMarker(false) => Kind::Extra(Extra::TaskUnchecked),
        Event::Start(_) | Event::End(_) => return None,
    })
}

/// Closes the innermost construct of `open` into the one around it; a section ends where its last block ends.
fn close(open: &mut Vec<(Kind, Walked)>) {
    if open.len() > 1
        && let Some((kind, mut done)) = open.pop()
        && let Some((_, parent)) = open.last_mut()
    {
        if kind == Kind::Extra(Extra::Section)
            && let Some(last) = done.children.last()
        {
            done.end = last.end;
        }
        parent.children.push(done);
    }
}

/// How deep a heading of `level` is, from 1: the walk's own table.
const fn depth(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Closes the sections of `sections` that `holds` says end, each one the innermost construct of `open`.
fn close_sections(
    open: &mut Vec<(Kind, Walked)>,
    sections: &mut Vec<(u8, usize)>,
    holds: impl Fn((u8, usize)) -> bool,
) {
    while let Some(&last) = sections.last() {
        if holds(last) {
            break;
        }
        let _: Option<(u8, usize)> = sections.pop();
        close(open);
    }
}

/// The tree the events open and close, with the tokens of the events in it and nothing between them.
fn nested(text: &str) -> Walked {
    let root = piece(Kind::Node(MarkdownNode::Root), Leafness::Node, 0, text.len());
    let mut open = vec![(Kind::Node(MarkdownNode::Root), root)];
    // The sections open: how deep each heading is, and how many constructs were open when it started.
    let mut sections: Vec<(u8, usize)> = Vec::new();
    let mut constructs = 0_usize;
    for (event, range) in Parser::new_ext(text, OPTIONS).into_offset_iter() {
        let within = open
            .last()
            .map_or(Kind::Node(MarkdownNode::Root), |(kind, _)| *kind);
        match &event {
            Event::Start(tag) => {
                if let Tag::Heading { level, .. } = tag {
                    let level = depth(*level);
                    close_sections(&mut open, &mut sections, |(deep, at)| {
                        at < constructs || (at == constructs && deep < level)
                    });
                    let section = Kind::Extra(Extra::Section);
                    open.push((
                        section,
                        piece(section, Leafness::Node, range.start, range.start),
                    ));
                    sections.push((level, constructs));
                }
                let kind = construct(tag);
                open.push((kind, piece(kind, Leafness::Node, range.start, range.end)));
                constructs += 1;
            }
            Event::End(_) => {
                close_sections(&mut open, &mut sections, |(_, at)| at < constructs);
                close(&mut open);
                constructs = constructs.saturating_sub(1);
            }
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
            | Event::TaskListMarker(_) => {
                if let Some(kind) = leaf_kind(&event, within)
                    && let Some((_, top)) = open.last_mut()
                {
                    top.children
                        .push(piece(kind, Leafness::Token, range.start, range.end));
                }
            }
        }
    }
    while open.len() > 1 {
        close(&mut open);
    }
    open.pop().map_or_else(
        || piece(Kind::Node(MarkdownNode::Root), Leafness::Node, 0, text.len()),
        |(_, document)| document,
    )
}

/// Every edge of a node or token under `node`, with its own.
fn edges(node: &Walked, into: &mut Vec<Offset>) {
    into.push(node.start);
    into.push(node.end);
    for child in &node.children {
        edges(child, into);
    }
}

/// The stretches of tokens under `node`.
fn spelled(node: &Walked, into: &mut Vec<(Offset, Offset)>) {
    for child in &node.children {
        match child.leaf {
            Leafness::Token => into.push((child.start, child.end)),
            Leafness::Node => spelled(child, into),
        }
    }
}

/// Puts the token `gap` in the deepest node under `node` whose bytes hold it, before the first piece that starts where it ends or
/// later.
fn place(node: &mut Walked, gap: Walked) {
    let inner = node.children.iter().position(|child| {
        child.leaf == Leafness::Node && child.start <= gap.start && gap.end <= child.end
    });
    if let Some(at) = inner
        && let Some(child) = node.children.get_mut(at)
    {
        place(child, gap);
        return;
    }
    let at = node
        .children
        .iter()
        .position(|child| gap.end <= child.start)
        .unwrap_or(node.children.len());
    node.children.insert(at, gap);
}

/// The library's tree of `text`: the constructs its events open and close, the tokens of its events and of its link reference
/// definitions, and the bytes between them.
#[must_use]
pub(crate) fn walked(text: &str) -> Walked {
    let mut tree = nested(text);
    // A link reference definition gives no event: the library lists it beside the stream, and its bytes are one token.
    for (_, definition) in Parser::new_ext(text, OPTIONS)
        .reference_definitions()
        .iter()
    {
        place(
            &mut tree,
            piece(
                Kind::Node(MarkdownNode::Definition),
                Leafness::Token,
                definition.span.start,
                definition.span.end,
            ),
        );
    }
    let mut cuts = Vec::new();
    edges(&tree, &mut cuts);
    cuts.sort_unstable();
    cuts.dedup();
    let mut tokens = Vec::new();
    spelled(&tree, &mut tokens);
    let bytes = text.as_bytes();
    for pair in cuts.windows(2) {
        let (Some(&from), Some(&to)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        if tokens
            .iter()
            .any(|&(start, end)| start <= from && to <= end)
        {
            continue;
        }
        let stretch = bytes
            .get(
                usize::try_from(from.0).unwrap_or(usize::MAX)
                    ..usize::try_from(to.0).unwrap_or(usize::MAX),
            )
            .unwrap_or_default();
        let kind = if stretch.iter().all(|byte| WHITE_SPACE.contains(byte)) {
            Kind::Token(Token::Whitespace)
        } else {
            Kind::Extra(Extra::Markup)
        };
        place(
            &mut tree,
            Walked {
                kind: kind_bits(kind),
                leaf: Leafness::Token,
                start: from,
                end: to,
                children: Vec::new(),
            },
        );
    }
    tree
}

/// A node of mdast's tree: its type, and the nodes under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Shape {
    /// The type, as mdast names it: `listItem`.
    pub name: String,
    /// The nodes under it, in order.
    pub children: Vec<Shape>,
}

/// A node of type `name` with `children` under it.
#[must_use]
pub(crate) fn shape(name: &str, children: Vec<Shape>) -> Shape {
    Shape {
        name: name.to_owned(),
        children,
    }
}

/// The depth of a printed line and the type of the node it prints; nothing for a line that prints a field of a node. A node line is
/// the branch (`├─` or `└─`), the node's place among its siblings, and its type, after the pipes and spaces of its ancestors.
fn printed(line: &str) -> Option<(usize, &str)> {
    let rest = line.trim_start_matches(['│', ' ']);
    let after = rest
        .strip_prefix("├─")
        .or_else(|| rest.strip_prefix("└─"))?;
    let named = after
        .trim_start_matches(|each: char| each.is_ascii_digit())
        .strip_prefix(' ')?
        .trim_start();
    let end = named.find([' ', '[']).unwrap_or(named.len());
    let depth = (line.chars().count() - rest.chars().count()) / 4 + 1;
    Some((depth, named.get(..end)?))
}

/// The tree `unist-util-inspect` printed, read back: each line that prints a node is a node of its depth, the fields (`depth:`,
/// `url:`), the values and the counts dropped. Nothing when the text is no print of a tree.
#[must_use]
pub(crate) fn dumped(print: &str) -> Option<Shape> {
    let mut lines = print.lines();
    let first = lines.next()?;
    let root = first
        .find(['[', ' '])
        .and_then(|end| first.get(..end))
        .unwrap_or(first);
    let mut open: Vec<Shape> = vec![shape(root, Vec::new())];
    for line in lines {
        let Some((depth, name)) = printed(line) else {
            continue;
        };
        while open.len() > depth {
            let done = open.pop()?;
            open.last_mut()?.children.push(done);
        }
        open.push(shape(name, Vec::new()));
    }
    while open.len() > 1 {
        let done = open.pop()?;
        open.last_mut()?.children.push(done);
    }
    open.pop()
}

/// How a shape is written: `name(child child)`.
fn written(node: &Shape) -> String {
    let children: Vec<String> = node.children.iter().map(written).collect();
    format!("{}({})", node.name, children.join(" "))
}

/// What differs between our projected tree and the standard's, the first place found; nothing when they agree.
#[must_use]
pub(crate) fn differences(ours: &Shape, theirs: &Shape, path: &str) -> Option<String> {
    let here = format!("{path}/{}", ours.name);
    if ours.name != theirs.name {
        return Some(format!(
            "{path}: we have `{}`, mdast has `{}`",
            ours.name, theirs.name
        ));
    }
    for (mine, yours) in ours.children.iter().zip(&theirs.children) {
        if let Some(found) = differences(mine, yours, &here) {
            return Some(found);
        }
    }
    (ours.children.len() != theirs.children.len()).then(|| {
        format!(
            "{here}: we have {}, mdast has {}",
            written(ours),
            written(theirs)
        )
    })
}

#[cfg(test)]
#[path = "walk_test.rs"]
mod tests;
