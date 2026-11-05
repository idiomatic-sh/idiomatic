//! The syntax port, implemented with `tree_sitter_go`, the Go grammar of tree-sitter. The library's tree is a concrete one: every
//! keyword, bracket and comma is a node of it, and the groups it makes are not the ones `go/ast` makes. The adapter reads it into plain
//! data, in the words of Go's own syntax, and the domain finds in it the nodes of `go/ast` and the tokens of `go/token`; they are then
//! written into the element table in the order of the file: a token is under the innermost node that holds it, and the text between
//! two tokens is a `whitespace` token under the node that holds it, so the tokens of the table spell the file.
//!
//! What the library says of the file is the text it could not read, the text it expected, and what it reads where Go does not allow
//! it: each a diagnostic. The library's names stop here; what leaves is the language's own `Kind`.

use ryo::languages::{self, ByteSlice, Offset, ReportText};
use tree_sitter::{Node as Library, Parser, TreeCursor};

use crate::domain::extras::types::Extra;
use crate::domain::grammar::types::{
    ByteIndex, Concrete, Form, GoText, Index, Naming, Presence, Raw,
};
use crate::domain::nodes::types::Node;
use crate::domain::pieces::service::{arrange, value_of};
use crate::domain::pieces::types::Pieces;
use crate::domain::shapes::service::find;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;

use super::constants::{
    BYTE_ORDER_MARK, MISSING, NEEDS_COMMA, NOT_UTF8, OUTSIDE_FUNCTION, UNREADABLE, UNTERMINATED,
    WHITE_SPACE,
};
use super::schemas::{closing, form_of, misreading, role_of};
use super::types::{Base, Misreading, Report, Said};

/// The `tree_sitter_go` syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxTreeSitterGoAdapter;

/// The offset of a byte index of the file.
fn offset(index: ByteIndex) -> Offset {
    Offset(u32::try_from(index.0).unwrap_or(u32::MAX))
}

/// A parser for Go.
///
/// # Errors
///
/// What the library says when the grammar and the library disagree on their versions.
pub(super) fn parser() -> Result<Parser, tree_sitter::LanguageError> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_go::LANGUAGE.into())?;
    Ok(parser)
}

/// What the library says of the node `node`, besides what it is.
fn report(node: &Library<'_>, form: Form) -> Option<Report> {
    let (start, end) = (ByteIndex(node.start_byte()), ByteIndex(node.end_byte()));
    let said = if node.is_missing() {
        Said::Missing
    } else if node.is_error() {
        Said::Unreadable
    } else if form == Form::Text && node.has_error() {
        Said::Unterminated
    } else {
        return match misreading(node) {
            Misreading::Statement => Some(Report {
                start,
                end,
                said: Said::OutsideFunction,
            }),
            Misreading::Comma => {
                let last = closing(node).unwrap_or(*node);
                Some(Report {
                    start: ByteIndex(last.start_byte()),
                    end: ByteIndex(last.end_byte()),
                    said: Said::NeedsComma,
                })
            }
            Misreading::Fits => None,
        };
    };
    Some(Report { start, end, said })
}

/// Puts the node `node` of the library, which the cursor is at, in the table of nodes, under `parent`.
fn push(nodes: &mut Vec<Raw>, cursor: &TreeCursor<'_>, parent: Option<Index>, form: Form) -> Index {
    let node = cursor.node();
    let at = Index(nodes.len());
    let previous = parent.and_then(|each| {
        let up = nodes.get(each.0)?;
        up.last
    });
    nodes.push(Raw {
        form,
        role: role_of(cursor.field_name()),
        naming: if node.is_named() {
            Naming::Named
        } else {
            Naming::Anonymous
        },
        presence: if node.is_missing() {
            Presence::Missing
        } else {
            Presence::Read
        },
        start: ByteIndex(node.start_byte()),
        end: ByteIndex(node.end_byte()),
        parent,
        first: None,
        last: None,
        next: None,
        previous,
    });
    if let Some(up) = parent.and_then(|each| nodes.get_mut(each.0)) {
        up.last = Some(at);
        if previous.is_none() {
            up.first = Some(at);
        }
    }
    if let Some(before) = previous.and_then(|each| nodes.get_mut(each.0)) {
        before.next = Some(at);
    }
    at
}

/// The library's tree of a file as plain data in the words of Go, and what the library says of it, in the order it meets the nodes.
pub(super) fn concrete<'t>(
    tree: &tree_sitter::Tree,
    text: GoText<'t>,
    reports: &mut Vec<Report>,
) -> Concrete<'t> {
    let mut nodes = Vec::new();
    let mut cursor = tree.walk();
    let mut open: Vec<Index> = Vec::new();
    loop {
        let node = cursor.node();
        let form = form_of(&node);
        let at = push(&mut nodes, &cursor, open.last().copied(), form);
        reports.extend(report(&node, form));
        // A string holds its content and its escapes in the token of its own.
        if form != Form::Text && cursor.goto_first_child() {
            open.push(at);
            continue;
        }
        while !cursor.goto_next_sibling() {
            if open.pop().is_none() || !cursor.goto_parent() {
                return Concrete { nodes, text };
            }
        }
    }
}

/// Writes the text between two tokens: white space, or text the library could not read, as the tokens it is made of.
fn gap(
    readable: ByteIndex,
    bytes: SourceText<'_>,
) -> impl FnMut(&mut languages::TableBuilder<'_>, Offset, Offset) + '_ {
    move |builder, from, to| {
        let (mut at, stop) = (
            usize::try_from(from.0).unwrap_or(usize::MAX),
            usize::try_from(to.0).unwrap_or(usize::MAX),
        );
        // The byte order mark that opens a file is white space, as `go/scanner` reads it.
        let mark = BYTE_ORDER_MARK.len_utf8();
        if at == 0
            && stop >= mark
            && bytes
                .0
                .starts_with(BYTE_ORDER_MARK.encode_utf8(&mut [0; 4]).as_bytes())
        {
            languages::token(
                builder,
                kind_bits(Kind::Extra(Extra::Whitespace)),
                Offset(0),
                offset(ByteIndex(mark)),
            );
            at = mark;
        }
        while at < stop {
            let read = at < readable.0;
            let limit = if read { stop.min(readable.0) } else { stop };
            let space = |byte: &_| WHITE_SPACE.contains(byte);
            let first = bytes.0.get(at).copied().unwrap_or(b' ');
            let kind = if !read {
                Kind::Extra(Extra::Error)
            } else if space(&first) {
                Kind::Extra(Extra::Whitespace)
            } else {
                Kind::Token(Token::Illegal)
            };
            let mut next = at + 1;
            if read {
                while next < limit && bytes.0.get(next).is_some_and(space) == space(&first) {
                    next += 1;
                }
            } else {
                next = limit;
            }
            languages::token(
                builder,
                kind_bits(kind),
                offset(ByteIndex(at)),
                offset(ByteIndex(next)),
            );
            at = next;
        }
    }
}

/// Closes the open nodes that end at or before `upto`.
fn close_nodes(
    open: &mut Vec<Offset>,
    written: &mut languages::Lossless<'_, '_>,
    upto: Offset,
    unread: &mut impl FnMut(&mut languages::TableBuilder<'_>, Offset, Offset),
) {
    while open.last().is_some_and(|end| *end <= upto) {
        if let Some(end) = open.pop() {
            languages::lossless_leave(written, end, unread);
        }
    }
}

/// The file `bytes` as the table of its root and what `found` holds under it, in the order of the file: a token is under the innermost
/// node that holds it, and the text between two tokens, under the node that holds it. `readable` is how many bytes of `bytes` the
/// library read.
fn write_tree(
    found: &Pieces,
    base: Base,
    bytes: SourceText<'_>,
    readable: ByteIndex,
    builder: &mut languages::TableBuilder<'_>,
) {
    let length = offset(ByteIndex(bytes.0.len()));
    let base = base.0;
    let shifted = |index: ByteIndex| offset(ByteIndex(index.0 + base));
    let mut unread = gap(readable, bytes);
    let mut written = languages::lossless(builder, Offset(0));
    languages::lossless_enter(
        &mut written,
        kind_bits(Kind::Node(Node::File)),
        Offset(0),
        &mut unread,
    );
    let mut open: Vec<Offset> = Vec::new();
    let (mut next_node, mut next_lexeme) = (0_usize, 0_usize);
    loop {
        let node = found.nodes.get(next_node);
        let lexeme = found.lexemes.get(next_lexeme);
        let nodes_first = match (node, lexeme) {
            (Some(each), Some(other)) => each.start <= other.start,
            (Some(_), None) => true,
            (None, _) => false,
        };
        let start = match (node, lexeme) {
            (Some(each), _) if nodes_first => shifted(each.start),
            (_, Some(each)) => shifted(each.start),
            (_, None) => break,
        };
        // The nodes that end where the last token ended are closed before the white space after it, and those that end where the next
        // element starts before that element.
        let last = written.end;
        close_nodes(&mut open, &mut written, last, &mut unread);
        languages::fill_to(&mut written, start, &mut unread);
        close_nodes(&mut open, &mut written, start, &mut unread);
        match (nodes_first, node, lexeme) {
            (true, Some(each), _) => {
                languages::lossless_enter(
                    &mut written,
                    kind_bits(Kind::Node(each.kind)),
                    shifted(each.start),
                    &mut unread,
                );
                open.push(shifted(each.end));
                next_node += 1;
            }
            (false, _, Some(each)) => {
                let bits = kind_bits(Kind::Token(each.kind));
                let (from, to) = (shifted(each.start), shifted(each.end));
                match value_of(found, each) {
                    Some(text) => {
                        languages::lossless_decoded_token(
                            &mut written,
                            bits,
                            from,
                            to,
                            text,
                            &mut unread,
                        );
                    }
                    None => {
                        languages::lossless_token(&mut written, bits, from, to, &mut unread);
                    }
                }
                next_lexeme += 1;
            }
            (true, None, _) | (false, _, None) => break,
        }
    }
    close_nodes(&mut open, &mut written, Offset(u32::MAX), &mut unread);
    languages::fill_to(&mut written, length, &mut unread);
    languages::lossless_leave(&mut written, length, &mut unread);
}

/// Writes what the library said, in the file's own places.
fn say(builder: &mut languages::TableBuilder<'_>, base: Base, reports: &[Report]) {
    for each in reports {
        let message = match each.said {
            Said::Unreadable => UNREADABLE,
            Said::Missing => MISSING,
            Said::OutsideFunction => OUTSIDE_FUNCTION,
            Said::NeedsComma => NEEDS_COMMA,
            Said::Unterminated => UNTERMINATED,
        };
        languages::diagnose(
            builder,
            ReportText(message),
            offset(ByteIndex(each.start.0 + base.0)),
            offset(ByteIndex(each.end.0 + base.0)),
        );
    }
}

impl SyntaxPort for SyntaxTreeSitterGoAdapter {
    fn read(&self, text: SourceText<'_>, builder: &mut languages::TableBuilder<'_>) {
        let readable = languages::readable_text(ByteSlice(text.0)).0;
        let (base, content) = match readable.strip_prefix(BYTE_ORDER_MARK) {
            Some(rest) => (Base(BYTE_ORDER_MARK.len_utf8()), rest),
            None => (Base(0), readable),
        };
        let mut reports = Vec::new();
        let mut pieces = Pieces::default();
        // A grammar the library refuses reads nothing of the file: the file says so, and its tree is its text alone.
        let (parsed, refused) = match parser() {
            Ok(mut each) => (each.parse(content, None), None),
            Err(error) => (None, Some(error.to_string())),
        };
        if let Some(tree) = parsed {
            let concrete = concrete(&tree, GoText(content), &mut reports);
            find(&concrete, &mut pieces);
            arrange(&mut pieces);
        }
        write_tree(&pieces, base, text, ByteIndex(readable.len()), builder);
        if let Some(reason) = &refused {
            languages::diagnose(
                builder,
                ReportText(reason),
                offset(ByteIndex(0)),
                offset(ByteIndex(readable.len())),
            );
        }
        if readable.len() < text.0.len() {
            languages::diagnose(
                builder,
                ReportText(NOT_UTF8),
                offset(ByteIndex(readable.len())),
                offset(ByteIndex(text.0.len())),
            );
        }
        say(builder, base, &reports);
    }
}
