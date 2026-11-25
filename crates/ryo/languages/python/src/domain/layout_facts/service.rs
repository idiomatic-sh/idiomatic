//! The `layout_facts` service: free functions over the tokens of a walked file, which follow its nodes in the table in the order
//! of the file. The links between the tokens (each token's next, a bracket's matching closer, the innermost open bracket around
//! a token), the indentation regions the `INDENT` and `DEDENT` tokens open and close, and the line and column of an offset.

use super::types::{FileLines, LinePlace, LinkIndex, TokenAt};
use crate::domain::indentation::types::IndentationRegion;
use crate::domain::model::lexical::constants::NO_NODE;
use crate::domain::node::service::own_end;
use crate::domain::node::types::SyntaxFile;
use crate::domain::syntax::service::kind_of;
use crate::domain::syntax::types::Kind;
use crate::domain::token_links::types::{LinkKind, Opener, Pairing, TokenLink};
use crate::domain::tokens::types::Token;
use ryo::languages::{Integer, Offset, Position, Span, SyntaxCompleteness, node_table};

/// The tokens of the file's own tree, in the order of the file: the ids after its nodes. The white space between tokens is no
/// token of the language's.
pub(crate) fn tokens<'f>(file: &SyntaxFile<'f>) -> impl Iterator<Item = TokenAt> + use<'f> {
    let table = file.table;
    let from = u32::try_from(node_table::node_count(table).0).unwrap_or(u32::MAX);
    let to = own_end(file).0;
    (from..to).filter_map(
        move |id| match kind_of(node_table::kind_of(table, Position(id))) {
            Kind::Token(token) => Some(TokenAt {
                id: Position(id),
                token,
            }),
            Kind::Node(_) | Kind::Extra(_) => None,
        },
    )
}

/// The closing bracket of an opening one.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the three opening brackets have a closing token; every other token closes nothing"
)]
const fn closing(open: Token) -> Option<Token> {
    match open {
        Token::Lpar => Some(Token::Rpar),
        Token::Lsqb => Some(Token::Rsqb),
        Token::Lbrace => Some(Token::Rbrace),
        _ => None,
    }
}

/// Hands `each` every link between the tokens of the file, in the order of the file: a token's next token, an opening bracket's
/// matching closer, and the innermost open bracket around a token. Says whether the pairing held to the end: a token the lexer
/// could not read or a bracket that closes nothing loses it from there on, and a bracket left open loses it at the end.
pub(crate) fn token_links(
    file: &SyntaxFile<'_>,
    stack: &mut Vec<Opener>,
    each: &mut dyn FnMut(TokenLink),
) -> Pairing {
    stack.clear();
    let mut pairing = Pairing::Whole;
    let mut previous: Option<Position> = None;
    for TokenAt { id, token } in tokens(file) {
        if token == Token::Errortoken {
            pairing = Pairing::Lost;
            previous = None;
            stack.clear();
            continue;
        }
        let mut paired = None;
        let mut enclosing = stack.last().map(|opener| opener.at);
        if closing(token).is_some() {
            stack.push(Opener { token, at: id });
        } else if matches!(token, Token::Rpar | Token::Rsqb | Token::Rbrace) {
            match stack.pop() {
                Some(opener) if closing(opener.token) == Some(token) => {
                    paired = Some(opener.at);
                    enclosing = stack.last().map(|around| around.at);
                }
                Some(_) | None => {
                    pairing = Pairing::Lost;
                    stack.clear();
                    enclosing = None;
                }
            }
        } else {
            // Any other token opens and closes nothing: it is linked to its neighbours and to the bracket around it.
        }
        if let Some(source) = previous.replace(id) {
            each(TokenLink {
                source,
                target: id,
                kind: LinkKind::Next,
            });
        }
        if let Some(source) = paired {
            each(TokenLink {
                source,
                target: id,
                kind: LinkKind::Matching,
            });
        }
        if let Some(target) = enclosing {
            each(TokenLink {
                source: id,
                target,
                kind: LinkKind::Enclosing,
            });
        }
    }
    if !stack.is_empty() {
        pairing = Pairing::Lost;
        stack.clear();
    }
    pairing
}

/// Writes into `index` what each token of the file is linked to, in one pass over its tokens: the links are the ones
/// `token_links` hands, kept by token.
pub(crate) fn link_index(file: &SyntaxFile<'_>, stack: &mut Vec<Opener>, index: &mut LinkIndex) {
    let first = u32::try_from(node_table::node_count(file.table).0).unwrap_or(u32::MAX);
    let places = usize::from(Position(own_end(file).0.saturating_sub(first)));
    index.first = Position(first);
    for column in [&mut index.next, &mut index.previous, &mut index.pair, &mut index.around] {
        column.clear();
        column.resize(places, NO_NODE);
    }
    let put = |column: &mut Vec<Position>, at: Position, linked: Position| {
        if let Some(slot) = column.get_mut(usize::from(Position(at.0.saturating_sub(first)))) {
            *slot = linked;
        }
    };
    let (next, previous, pair, around) =
        (&mut index.next, &mut index.previous, &mut index.pair, &mut index.around);
    index.pairing = token_links(file, stack, &mut |link| match link.kind {
        LinkKind::Next => {
            put(next, link.source, link.target);
            put(previous, link.target, link.source);
        }
        LinkKind::Matching => {
            put(pair, link.source, link.target);
            put(pair, link.target, link.source);
        }
        LinkKind::Enclosing => put(around, link.source, link.target),
    });
}

/// What the token `id` is linked to in one column of the index; none where it has no such link, and for an id that is no token
/// of the file's own tree.
fn linked(index: &LinkIndex, column: &[Position], id: Position) -> Option<Position> {
    let place = id.0.checked_sub(index.first.0)?;
    column.get(usize::from(Position(place))).copied().filter(|found| *found != NO_NODE)
}

/// Hands `each` the links whose source is the token `id`, in the order `token_links` hands them: the bracket around it, the
/// token after it, and its closing bracket when it opens one. Three reads of the index.
pub(crate) fn links_from(index: &LinkIndex, id: Position, each: &mut dyn FnMut(TokenLink)) {
    if let Some(target) = linked(index, &index.around, id) {
        each(TokenLink { source: id, target, kind: LinkKind::Enclosing });
    }
    if let Some(target) = linked(index, &index.next, id) {
        each(TokenLink { source: id, target, kind: LinkKind::Next });
    }
    if let Some(target) = linked(index, &index.pair, id).filter(|closer| *closer > id) {
        each(TokenLink { source: id, target, kind: LinkKind::Matching });
    }
}

/// Hands `each` the links whose target is the token `id`, in the order `token_links` hands them: the token before it, its
/// opening bracket when it closes one, and, when it opens a bracket, each token it is the innermost open bracket around. It
/// steps over the brackets nested in it, so it reads the tokens directly in it and no other.
pub(crate) fn links_to(index: &LinkIndex, id: Position, each: &mut dyn FnMut(TokenLink)) {
    if let Some(source) = linked(index, &index.previous, id) {
        each(TokenLink { source, target: id, kind: LinkKind::Next });
    }
    if let Some(source) = linked(index, &index.pair, id).filter(|opener| *opener < id) {
        each(TokenLink { source, target: id, kind: LinkKind::Matching });
    }
    let mut inside = linked(index, &index.next, id);
    while let Some(source) = inside {
        if linked(index, &index.around, source) != Some(id) {
            break;
        }
        each(TokenLink { source, target: id, kind: LinkKind::Enclosing });
        // A bracket nested here that closes: its closer is around this bracket too, and what it holds is not.
        let closer = linked(index, &index.pair, source).filter(|closer| *closer > source);
        if let Some(closer) = closer
            && linked(index, &index.around, closer) == Some(id)
        {
            each(TokenLink { source: closer, target: id, kind: LinkKind::Enclosing });
        }
        inside = linked(index, &index.next, closer.unwrap_or(source));
    }
}

/// The indentation regions of the file into `regions`, ordered by their ids: the module is the region of the root, and every
/// `INDENT` token opens a region its `DEDENT` closes. Says whether the regions are whole: they are not when the lexer did not read
/// every token, or when the indents and dedents do not pair.
pub(crate) fn indent_regions(
    file: &SyntaxFile<'_>,
    regions: &mut Vec<IndentationRegion>,
) -> SyntaxCompleteness {
    regions.clear();
    if file.completeness.lexical == SyntaxCompleteness::Incomplete {
        return SyntaxCompleteness::Incomplete;
    }
    let Ok(length) = u32::try_from(file.text.0.len()) else {
        return SyntaxCompleteness::Incomplete;
    };
    regions.push(IndentationRegion {
        id: Position(0),
        parent: Position(0),
        span: Span {
            start: Offset(0),
            end: Offset(length),
        },
        indentation_end: Offset(0),
    });
    let mut open: Vec<IndentationRegion> = Vec::new();
    for TokenAt { id, token } in tokens(file) {
        let span = node_table::span_of(file.table, id);
        if token == Token::Indent {
            if span.start > span.end || span.end.0 > length {
                return SyntaxCompleteness::Incomplete;
            }
            open.push(IndentationRegion {
                id,
                parent: open.last().map_or(Position(0), |region| region.id),
                span: Span {
                    start: span.start,
                    end: span.end,
                },
                indentation_end: span.end,
            });
        } else if token == Token::Dedent {
            let Some(mut region) = open.pop() else {
                return SyntaxCompleteness::Incomplete;
            };
            if span.end < region.indentation_end || span.end.0 > length {
                return SyntaxCompleteness::Incomplete;
            }
            region.span.end = span.end;
            regions.push(region);
        } else if token == Token::Errortoken && (span.start != span.end || span.end.0 != length) {
            // The lexer's end-of-file marker is the one error token a whole token stream holds: any other consumed source.
            return SyntaxCompleteness::Incomplete;
        } else {
            // Any other token opens and closes no region.
        }
    }
    if !open.is_empty() {
        return SyntaxCompleteness::Incomplete;
    }
    regions.sort_unstable_by_key(|region| region.id);
    SyntaxCompleteness::Complete
}

/// Where each line of the file starts, into `lines`.
pub(crate) fn line_starts(file: &SyntaxFile<'_>, lines: &mut FileLines) {
    lines.0.clear();
    lines.0.push(Offset(0));
    let bytes = file.text.0.as_bytes();
    lines.0.extend(bytes.iter().enumerate().filter_map(|(at, &byte)| {
        let next = at + 1;
        let newline = byte == b'\n' || (byte == b'\r' && bytes.get(next) != Some(&b'\n'));
        newline.then(|| Offset(u32::try_from(next).unwrap_or(u32::MAX)))
    }));
}

/// The line and the column of an offset the caller expects at or after line `near` (a 1-based line it already holds):
/// the search starts there and doubles its step, so the nearer the offset, the fewer starts it compares. The answer is the
/// same as `line_place` for any `near`.
#[must_use]
pub(crate) fn line_place_near(lines: &FileLines, offset: Offset, near: usize) -> LinePlace {
    let starts = &lines.0;
    let from = near.saturating_sub(1).min(starts.len().saturating_sub(1));
    let line = if starts.get(from).is_some_and(|start| *start <= offset) {
        let mut low = from;
        let mut step = 1;
        while starts.get(low + step).is_some_and(|start| *start <= offset) {
            low += step;
            step *= 2;
        }
        let high = (low + step).min(starts.len());
        low + 1 + starts[low + 1..high].partition_point(|start| *start <= offset)
    } else {
        starts.partition_point(|start| *start <= offset)
    }
    .max(1);
    let start = starts.get(line - 1).copied().unwrap_or(Offset(0));
    LinePlace {
        line: Integer(i64::try_from(line).unwrap_or(i64::MAX)),
        column: Integer(i64::from(offset.0.saturating_sub(start.0))),
    }
}

/// The line and the column of an offset of the file.
#[must_use]
pub(crate) fn line_place(lines: &FileLines, offset: Offset) -> LinePlace {
    let line = lines.0.partition_point(|start| *start <= offset).max(1);
    let start = lines.0.get(line - 1).copied().unwrap_or(Offset(0));
    LinePlace {
        line: Integer(i64::try_from(line).unwrap_or(i64::MAX)),
        column: Integer(i64::from(offset.0.saturating_sub(start.0))),
    }
}
