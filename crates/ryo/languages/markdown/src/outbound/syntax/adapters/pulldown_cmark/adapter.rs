//! The syntax port, implemented with `pulldown-cmark`, the `CommonMark` pull parser: its events are written straight into the element
//! table, with no document built. The parser gives a flat stream of events, each with the bytes it spans: a `Start` and an `End`
//! for each construct, and one event for each piece of text, code, HTML or break. The adapter opens a node for each construct and
//! writes a token for each other event; the bytes no event spans (the markers of blocks and spans, indentation, fences, link
//! destinations, the definitions of link references, blank lines) are tokens of their own, white space or markup, so the tokens
//! read back the whole file. A heading opens a section, a node the parser does not give, that holds it and the blocks after it up to
//! the next heading as deep or shallower among the same blocks, and ends where its last block ends. The code a code block holds and the code of a span are written with the text the library read from
//! them. The library's names stop here; what leaves is the language's own `Kind`.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ryo::languages::SourcePath;
use ryo::languages::{self, ByteSlice, Count, NameText, Offset, node_table};

use crate::domain::syntax::service::{field_bits, kind_bits};
use crate::domain::syntax::types::{Field, Kind, Level};
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;

use super::constants::{NOT_UTF8, OPTIONS, WHITE_SPACE};
use super::schemas::{depth_of, event_kind, level_of, tag_kind};
use super::interface::{Ops, Tree};
use super::types::{
    Carried, Closing, Definition, Gap, MarkdownText, Opened, Passed, Pending, Stretch, Within,
    Writer,
};
use super::walk::Numbered;

/// The `pulldown_cmark` syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxPulldownCmarkAdapter;

/// The offset of a byte index of the text.
fn offset(index: Count) -> Offset {
    Offset(u32::try_from(index.0).unwrap_or(u32::MAX))
}

/// Writes the bytes from `from`, where the last token ends, to `to`, which no event spans, as white space or markup: the gap tokenizer of
/// the tree. The definition of a link reference among them is a token of its own, with the label it gives.
pub(super) fn fill(gap: &mut Gap<'_>, builder: &mut impl Ops, from: Offset, to: Offset) {
    let mut end = from;
    while end < to {
        while gap
            .definitions
            .get(gap.passed.0)
            .is_some_and(|definition| definition.end <= end)
        {
            gap.passed.0 += 1;
        }
        let next = gap
            .definitions
            .get(gap.passed.0)
            .map(|definition| (definition.start, definition.end, definition.label.clone()));
        if let Some((_, stop_at, label)) = next.clone().filter(|(start, _, _)| *start <= end) {
            let stop = stop_at.min(to);
            builder.decoded(Kind::Node(Node::Definition), end, stop, &label.0);
            end = stop;
        } else if let Some(start) = next.map(|(start, _, _)| start).filter(|start| *start < to) {
            plain(gap, builder, end, start);
            end = start;
        } else {
            plain(gap, builder, end, to);
            end = to;
        }
    }
}

/// Writes the bytes from `from` to `to` as white space or markup; the markup that opens a construct which names something carries what
/// it names.
fn plain(gap: &mut Gap<'_>, builder: &mut impl Ops, from: Offset, to: Offset) {
    let bytes = gap
        .source
        .0
        .get(
            usize::try_from(from.0).unwrap_or(usize::MAX)
                ..usize::try_from(to.0).unwrap_or(usize::MAX),
        )
        .unwrap_or_default();
    if bytes.iter().all(|byte| WHITE_SPACE.contains(byte)) {
        builder.token(Kind::Token(Token::Whitespace), from, to);
    } else if let Pending::Carrying(named) = std::mem::replace(&mut gap.pending, Pending::Nothing) {
        builder.decoded(Kind::Extra(Extra::Markup), from, to, &named.0);
    } else {
        builder.token(Kind::Extra(Extra::Markup), from, to);
    }
}

/// Opens a construct, where its event starts, or where the last token ends when the event starts before it.
fn enter<T: Tree>(writer: &mut Writer<'_, T>, kind: Kind, stretch: Stretch) {
    writer.tree.enter(kind, offset(stretch.start), &mut writer.gap);
    writer.open.push(kind);
}

/// Closes the open sections that `holds` says end: each where its last block ends, so the bytes after it are its parent's.
fn close_sections<T: Tree>(writer: &mut Writer<'_, T>, closing: Closing) {
    while let Some(&last) = writer.sections.last() {
        let held = match closing {
            Closing::Heading { depth, level } => {
                last.depth < depth || (last.depth == depth && last.level < level)
            }
            Closing::Within(depth) => last.depth < depth,
            Closing::Every => false,
        };
        if held {
            break;
        }
        let _: Option<Opened> = writer.sections.pop();
        writer.tree.leave();
    }
}

/// Opens the section of a heading `level` deep, where the heading starts, after closing the sections among the same blocks that
/// are as deep or deeper.
fn open_section<T: Tree>(writer: &mut Writer<'_, T>, level: Level, stretch: Stretch) {
    let depth = Count(writer.open.len());
    close_sections(writer, Closing::Heading { depth, level });
    writer.tree.enter(Kind::Extra(Extra::Section), offset(stretch.start), &mut writer.gap);
    writer.sections.push(Opened { level, depth });
}

/// Closes the innermost construct, at the end of its event or of its last token, whichever is later: the sections among its blocks
/// first.
fn leave<T: Tree>(writer: &mut Writer<'_, T>, stretch: Stretch) {
    close_sections(writer, Closing::Within(Count(writer.open.len())));
    writer.tree.fill_to(offset(stretch.end), &mut writer.gap);
    writer.gap.pending = Pending::Nothing;
    if writer.open.pop().is_some() {
        writer.tree.leave();
    }
}

/// Writes an event of text as a token, with the text the library read from it when it is given.
fn leaf<T: Tree>(writer: &mut Writer<'_, T>, kind: Kind, stretch: Stretch, decoded: Option<NameText<'_>>) {
    let (start, end) = (offset(stretch.start), offset(stretch.end));
    match decoded {
        Some(text) => writer.tree.decoded(kind, start, end, text.0, &mut writer.gap),
        None => writer.tree.token(kind, start, end, &mut writer.gap),
    }
}

/// Writes an event that is a piece of the document as the token or node of its kind, with the text the library read when it is given.
fn emit<T: Tree>(
    writer: &mut Writer<'_, T>,
    event: &Event<'_>,
    stretch: Stretch,
    decoded: Option<NameText<'_>>,
) {
    if let Some(kind) = event_kind(event, writer.within) {
        leaf(writer, kind, stretch, decoded);
    }
}

/// Writes one event of the parser.
fn event<T: Tree>(writer: &mut Writer<'_, T>, event: &Event<'_>, stretch: Stretch) {
    match event {
        Event::Start(tag) => {
            let within = if matches!(tag, Tag::CodeBlock(_)) {
                Some(Within::Code)
            } else if matches!(tag, Tag::HtmlBlock) {
                Some(Within::Indent)
            } else {
                None
            };
            if let Some(within) = within {
                writer.within = within;
            }
            if let Tag::Heading { level, .. } = tag {
                open_section(writer, Level(level_of(*level)), stretch);
            }
            enter(writer, tag_kind(tag), stretch);
            writer.gap.pending = if let Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. } =
                tag
            {
                Pending::Carrying(Carried(
                    languages::resolve_target(writer.location, languages::WrittenTarget(dest_url))
                        .0,
                ))
            } else if let Tag::FootnoteDefinition(label) = tag {
                Pending::Carrying(Carried(label.to_string()))
            } else if let Tag::Heading { level, .. } = tag {
                Pending::Carrying(Carried(depth_of(*level).to_owned()))
            } else {
                Pending::Nothing
            };
        }
        Event::End(end) => {
            if matches!(end, TagEnd::CodeBlock | TagEnd::HtmlBlock) {
                writer.within = Within::Prose;
            }
            leave(writer, stretch);
        }
        Event::Text(text) => {
            let decoded = (writer.within == Within::Code).then_some(NameText(text));
            emit(writer, event, stretch, decoded);
        }
        Event::Code(code) => emit(writer, event, stretch, Some(NameText(code))),
        Event::InlineMath(_)
        | Event::DisplayMath(_)
        | Event::Html(_)
        | Event::InlineHtml(_)
        | Event::FootnoteReference(_)
        | Event::SoftBreak
        | Event::HardBreak
        | Event::Rule
        | Event::TaskListMarker(_) => emit(writer, event, stretch, None),
    }
}

/// Reads `content`, the bytes of the file the parser reads, into `tree`, which holds the document node open, and closes the document.
/// `whole` is the length of the whole file.
fn events<T: Tree>(tree: T, text: SourceText<'_>, content: MarkdownText<'_>, at: SourcePath<'_>) -> T {
    let parser = Parser::new_ext(content.0, OPTIONS);
    let mut definitions: Vec<Definition> = parser
        .reference_definitions()
        .iter()
        .map(|(label, definition)| Definition {
            start: offset(Count(definition.span.start)),
            end: offset(Count(definition.span.end)),
            label: Carried(label.to_owned()),
        })
        .collect();
    definitions.sort_by_key(|definition| definition.start);
    let mut writer = Writer {
        tree,
        gap: Gap {
            source: ByteSlice(text.0),
            definitions,
            passed: Passed(0),
            pending: Pending::Nothing,
        },
        open: Vec::new(),
        within: Within::Prose,
        location: at,
        sections: Vec::new(),
    };
    for (each, range) in parser.into_offset_iter() {
        let stretch = Stretch {
            start: Count(range.start),
            end: Count(range.end),
        };
        event(&mut writer, &each, stretch);
    }
    let read = Count(content.0.len());
    while !writer.open.is_empty() {
        leave(
            &mut writer,
            Stretch {
                start: read,
                end: read,
            },
        );
    }
    close_sections(&mut writer, Closing::Every);
    writer
        .tree
        .finish(offset(read), offset(Count(text.0.len())), &mut writer.gap);
    writer.tree
}

impl SyntaxPort for SyntaxPulldownCmarkAdapter {
    fn read(
        &self,
        text: SourceText<'_>,
        at: SourcePath<'_>,
        builder: &mut languages::TableBuilder<'_>,
    ) {
        // Markdown is UTF-8: the library reads the text up to the first byte that is not, and the rest is text it could not read.
        let content = MarkdownText(languages::readable_text(ByteSlice(text.0)).0);
        languages::enter_node(builder, kind_bits(Kind::Node(Node::Root)), Offset(0));
        let _ = builder.decode_for(&[
            languages::Relation::StringLiteral, languages::Relation::Defines,
            languages::Relation::Reference, languages::Relation::Scope, languages::Relation::Prose,
        ]);
        let tree = events(languages::lossless(builder, Offset(0)), text, content, at);
        if content.0.len() < text.0.len() {
            languages::diagnose(
                tree.builder,
                languages::ReportText(NOT_UTF8),
                offset(Count(content.0.len())),
                offset(Count(text.0.len())),
            );
        }
    }

    fn walk(
        &self,
        text: SourceText<'_>,
        at: SourcePath<'_>,
        decodings: &mut crate::domain::file_view::types::Decodings,
        builder: &mut languages::NodeTableBuilder<'_>,
    ) {
        let content = MarkdownText(languages::readable_text(ByteSlice(text.0)).0);
        node_table::enter_node(
            builder,
            kind_bits(Kind::Node(Node::Root)),
            languages::FieldCode(field_bits(Field::None).0),
            Offset(0),
        );
        let tree = events(Numbered::new(builder, decodings), text, content, at);
        let builder = tree.builder;
        let accepted = content.0.len() == text.0.len();
        if !accepted {
            node_table::diagnose(
                builder,
                offset(Count(content.0.len())),
                offset(Count(text.0.len())),
                NameText(NOT_UTF8),
            );
        }
        node_table::set_syntax_acceptance(
            builder,
            if accepted {
                languages::SyntaxAcceptance::Accepted
            } else {
                languages::SyntaxAcceptance::Rejected
            },
        );
    }
}

/// The old element table's tokens, written by the gap tokenizer through the builder.
struct Old<'b, 'w>(&'b mut languages::TableBuilder<'w>);

impl Ops for Old<'_, '_> {
    fn token(&mut self, kind: Kind, from: Offset, to: Offset) {
        languages::token(self.0, kind_bits(kind), from, to);
    }

    fn decoded(&mut self, kind: Kind, from: Offset, to: Offset, text: &str) {
        languages::decoded_token(self.0, kind_bits(kind), from, to, NameText(text));
    }
}

impl Tree for languages::Lossless<'_, '_> {
    fn enter(&mut self, kind: Kind, start: Offset, gap: &mut Gap<'_>) {
        languages::lossless_enter(
            self,
            kind_bits(kind),
            start,
            &mut |builder: &mut languages::TableBuilder<'_>, from, to| fill(gap, &mut Old(builder), from, to),
        );
    }

    fn leave(&mut self) {
        languages::leave_node(self.builder, self.end);
    }

    fn fill_to(&mut self, to: Offset, gap: &mut Gap<'_>) {
        languages::fill_to(
            self,
            to,
            &mut |builder: &mut languages::TableBuilder<'_>, from, to| fill(gap, &mut Old(builder), from, to),
        );
    }

    fn token(&mut self, kind: Kind, start: Offset, end: Offset, gap: &mut Gap<'_>) {
        languages::lossless_token(
            self,
            kind_bits(kind),
            start,
            end,
            &mut |builder: &mut languages::TableBuilder<'_>, from, to| fill(gap, &mut Old(builder), from, to),
        );
    }

    fn decoded(&mut self, kind: Kind, start: Offset, end: Offset, text: &str, gap: &mut Gap<'_>) {
        languages::lossless_decoded_token(
            self,
            kind_bits(kind),
            start,
            end,
            NameText(text),
            &mut |builder: &mut languages::TableBuilder<'_>, from, to| fill(gap, &mut Old(builder), from, to),
        );
    }

    fn finish(&mut self, read: Offset, length: Offset, gap: &mut Gap<'_>) {
        Tree::fill_to(self, read, gap);
        languages::fill_to(
            self,
            length,
            &mut |builder: &mut languages::TableBuilder<'_>, from, to| {
                languages::token(builder, kind_bits(Kind::Extra(Extra::Error)), from, to);
            },
        );
        languages::leave_node(self.builder, length);
    }
}
