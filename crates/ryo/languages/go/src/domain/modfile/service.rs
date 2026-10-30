//! The `modfile` service: a module's `go.mod` read into the table as `golang.org/x/mod/modfile` reads it. The file is one node
//! (`mod-file`); a directive on one line is a `mod-line` of its words, and a verb followed by `(` opens a `mod-line-block` whose lines
//! are `mod-line`s up to its `)`. Every byte is a token: the words, the parentheses, the comments and the space between.

use ryo::languages::{self, Offset};

use super::constants::{COMMENT_OPEN, UNCLOSED};
use super::types::{Opened, Piece, PieceKind};
use crate::domain::extras::types::Extra;
use crate::domain::grammar::types::ByteIndex;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;

/// Reads the `go.mod` file `text` into the table.
pub(crate) fn read(text: languages::ByteSlice<'_>, builder: &mut languages::TableBuilder<'_>) {
    let bytes = text.0;
    let end = offset(ByteIndex(bytes.len()));
    languages::enter_node(builder, extra(Extra::ModFile), Offset(0));
    let mut opened = Opened::Closed;
    let mut start = 0;
    while start < bytes.len() {
        let stop = bytes
            .get(start..)
            .and_then(|rest| rest.iter().position(|byte| *byte == b'\n'))
            .map_or(bytes.len(), |at| start + at + 1);
        opened = line(
            builder,
            &pieces(
                languages::ByteSlice(bytes.get(start..stop).unwrap_or_default()),
                offset(ByteIndex(start)),
            ),
            opened,
        );
        start = stop;
    }
    // A block the file leaves open is closed with the file, and is an error, as `modfile` refuses it.
    if let Opened::Open(opened_at) = opened {
        languages::leave_node(builder, end);
        languages::diagnose(builder, languages::ReportText(UNCLOSED), opened_at, end);
    }
    languages::leave_node(builder, end);
}

/// The table's number of an extra.
fn extra(kind: Extra) -> languages::KindBits {
    kind_bits(Kind::Extra(kind))
}

/// A byte's place in the file, as an offset of the table.
fn offset(at: ByteIndex) -> Offset {
    Offset(u32::try_from(at.0).unwrap_or(u32::MAX))
}

/// The pieces of one line of the file, its end included, which starts at `base`.
fn pieces(text: languages::ByteSlice<'_>, base: Offset) -> Vec<Piece> {
    let line = text.0;
    let mut found = Vec::new();
    let mut at = 0;
    let stop = line.len();
    let byte = |index| line.get(index).copied().unwrap_or(b'\n');
    let comment_at = |index| {
        line.get(index..)
            .is_some_and(|rest| rest.starts_with(COMMENT_OPEN))
    };
    while at < stop {
        let first = byte(at);
        let (kind, end) = if matches!(first, b' ' | b'\t' | b'\r' | b'\n') {
            let mut end = at + 1;
            while end < stop && matches!(byte(end), b' ' | b'\t' | b'\r' | b'\n') {
                end += 1;
            }
            (PieceKind::Space, end)
        } else if comment_at(at) {
            let mut end = at;
            while end < stop && byte(end) != b'\n' {
                end += 1;
            }
            (PieceKind::Comment, end)
        } else if first == b'(' {
            (PieceKind::Open, at + 1)
        } else if first == b')' {
            (PieceKind::Close, at + 1)
        } else if first == b'"' || first == b'`' {
            // A quoted word runs to its closing quote, a `\"` inside an interpreted string read as part of it; an unclosed one runs
            // to the line's end, as `modfile` reports it.
            let mut end = at + 1;
            while end < stop && byte(end) != first && byte(end) != b'\n' {
                end += if first == b'"' && byte(end) == b'\\' {
                    2
                } else {
                    1
                };
            }
            (PieceKind::Word, (end + 1).min(stop))
        } else {
            let mut end = at + 1;
            while end < stop
                && !matches!(byte(end), b' ' | b'\t' | b'\r' | b'\n' | b'(' | b')')
                && !comment_at(end)
            {
                end += 1;
            }
            (PieceKind::Word, end)
        };
        let end = end.min(stop);
        found.push(Piece {
            kind,
            start: Offset(base.0.saturating_add(offset(ByteIndex(at)).0)),
            end: Offset(base.0.saturating_add(offset(ByteIndex(end)).0)),
        });
        at = end.max(at + 1);
    }
    found
}

/// Writes one line's pieces into the table, opening or closing a block where the line does, and says whether a block is open after it.
fn line(builder: &mut languages::TableBuilder<'_>, pieces: &[Piece], opened: Opened) -> Opened {
    let content = |piece: &&Piece| {
        matches!(
            piece.kind,
            PieceKind::Word | PieceKind::Open | PieceKind::Close
        )
    };
    let (Some(first), Some(last)) = (
        pieces.iter().position(|piece| content(&piece)),
        pieces.iter().rposition(|piece| content(&piece)),
    ) else {
        tokens(builder, pieces);
        return opened;
    };
    let (before, rest) = pieces.split_at(first);
    let (within, after) = rest.split_at(last - first + 1);
    let opening = within.last().map(|piece| piece.kind) == Some(PieceKind::Open);
    let closing =
        within.len() == 1 && within.first().map(|piece| piece.kind) == Some(PieceKind::Close);
    let from = within.first().map_or(Offset(0), |piece| piece.start);
    let to = within.last().map_or(Offset(0), |piece| piece.end);
    tokens(builder, before);
    match (opened, closing, opening) {
        (Opened::Open(_), true, _) => {
            tokens(builder, within);
            languages::leave_node(builder, to);
            tokens(builder, after);
            Opened::Closed
        }
        (Opened::Closed, _, true) => {
            languages::enter_node(builder, extra(Extra::ModLineBlock), from);
            tokens(builder, within);
            tokens(builder, after);
            Opened::Open(from)
        }
        (Opened::Open(_) | Opened::Closed, _, _) => {
            languages::enter_node(builder, extra(Extra::ModLine), from);
            tokens(builder, within);
            languages::leave_node(builder, to);
            tokens(builder, after);
            opened
        }
    }
}

/// Writes each piece as a token of its kind.
fn tokens(builder: &mut languages::TableBuilder<'_>, pieces: &[Piece]) {
    for piece in pieces {
        let kind = match piece.kind {
            PieceKind::Space => Extra::Whitespace,
            PieceKind::Word => Extra::ModWord,
            PieceKind::Open => Extra::ModLParen,
            PieceKind::Close => Extra::ModRParen,
            PieceKind::Comment => Extra::ModComment,
        };
        languages::token(builder, extra(kind), piece.start, piece.end);
    }
}
