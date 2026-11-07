//! The syntax port, implemented with `jsonc_parser`, the parser of dprint, Deno and swc for JSON and JSON with comments: its tokens and
//! the objects, members and arrays of its tree are written straight into the element table. The library reads the grammar of json
//! with comments, which every dialect shares, so the tree of a file is the same whichever dialect the file is read as; the dialect
//! decides only what the file is told: a file read as json as the standard defines it gets a diagnostic for each comment and each
//! trailing comma, and one for a file with no value. A file the library cannot read is a flat tree of the tokens the library scans
//! before it fails, then the text it could not read, with the library's one diagnostic. The tree is lossless: white space the library
//! skips is written as `ws` tokens, and text no token spans as text it could not read.
//!
//! The library's names stop in the schemas; what leaves is the language's own `Kind`.

use jsonc_parser::ast::Value;
use jsonc_parser::common::Range;
use jsonc_parser::errors::ParseError;
use jsonc_parser::tokens::{Token as Scanned, TokenAndRange};
use jsonc_parser::{
    CollectOptions, CommentCollectionStrategy, ParseOptions, ParseResult, Scanner, ScannerOptions,
    parse_to_ast,
};
use ryo::languages::{self, ByteSlice, Count, NameText, Offset};

use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Dialect;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;

use super::constants::{
    COMMENT_IN_JSON, CONTROL_IN_STRING, NO_VALUE, NOT_UTF8, TRAILING_COMMA_IN_JSON, UNREAD_TEXT,
};
use super::schemas::{piece_kind, token_piece};
use super::types::{JsoncText, Piece, Reporting, Writing};

/// The `jsonc_parser` syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxJsoncParserAdapter;

/// The offset of a byte index of the text.
fn offset(index: Count) -> Offset {
    Offset(u32::try_from(index.0).unwrap_or(u32::MAX))
}

/// Where a range of the library starts.
fn start_of(range: Range) -> Offset {
    offset(Count(range.start))
}

/// Where a range of the library ends.
fn end_of(range: Range) -> Offset {
    offset(Count(range.end))
}

/// What the library reads: the grammar of json with comments, and nothing the library adds beyond it.
fn grammar() -> ParseOptions {
    ParseOptions {
        allow_comments: true,
        allow_loose_object_property_names: false,
        allow_trailing_commas: true,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
        allow_bare_decimal_point_numbers: false,
        allow_non_finite_numbers: false,
        allow_extended_string_escapes: false,
    }
}

/// What the library scans: the same tokens the grammar reads.
fn scanning() -> ScannerOptions {
    ScannerOptions {
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
        allow_bare_decimal_point_numbers: false,
        allow_non_finite_numbers: false,
        allow_extended_string_escapes: false,
    }
}

/// Writes the gap tokenizer of the tree: the bytes between two tokens are white space, then, if anything is left, text the parser
/// could not read.
fn unread(
    text: JsoncText<'_>,
    reporting: Reporting,
    builder: &mut languages::TableBuilder<'_>,
    from: Offset,
    to: Offset,
) {
    let (start, end) = (
        usize::try_from(from.0).unwrap_or(usize::MAX),
        usize::try_from(to.0).unwrap_or(usize::MAX),
    );
    let blanks = text.0.get(start..end).map_or(0, |gap| {
        gap.len() - gap.trim_start_matches([' ', '\t', '\n', '\r']).len()
    });
    let middle = offset(Count(start.saturating_add(blanks)));
    if blanks > 0 {
        languages::token(
            builder,
            kind_bits(piece_kind(Piece::Whitespace)),
            from,
            middle,
        );
    }
    if middle < to {
        languages::token(builder, kind_bits(piece_kind(Piece::Unread)), middle, to);
        if reporting == Reporting::Told && text.0.get(start..end).is_some() {
            languages::diagnose(builder, languages::ReportText(UNREAD_TEXT), middle, to);
        }
    }
}

/// Opens a node of `piece` at `start`, after the gap before it.
fn enter(writing: &mut Writing<'_, '_, '_>, piece: Piece, start: Offset) {
    let (text, reporting) = (writing.text, writing.reporting);
    languages::lossless_enter(
        &mut writing.tree,
        kind_bits(piece_kind(piece)),
        start,
        &mut |builder: &mut languages::TableBuilder<'_>, from, to| {
            unread(text, reporting, builder, from, to);
        },
    );
}

/// Closes the innermost node at `end`.
fn leave(writing: &mut Writing<'_, '_, '_>, end: Offset) {
    let (text, reporting) = (writing.text, writing.reporting);
    languages::lossless_leave(
        &mut writing.tree,
        end,
        &mut |builder: &mut languages::TableBuilder<'_>, from, to| {
            unread(text, reporting, builder, from, to);
        },
    );
}

/// Writes a token the library scanned, a string with the text the library decoded from it.
fn token(writing: &mut Writing<'_, '_, '_>, scanned: &TokenAndRange<'_>) {
    let (text, reporting) = (writing.text, writing.reporting);
    let (kind, start, end) = (
        kind_bits(piece_kind(token_piece(&scanned.token))),
        start_of(scanned.range),
        end_of(scanned.range),
    );
    let mut gap = |builder: &mut languages::TableBuilder<'_>, from, to| {
        unread(text, reporting, builder, from, to);
    };
    match &scanned.token {
        Scanned::String(decoded) => {
            languages::lossless_decoded_token(
                &mut writing.tree,
                kind,
                start,
                end,
                NameText(decoded),
                &mut gap,
            );
        }
        Scanned::OpenBrace
        | Scanned::CloseBrace
        | Scanned::OpenBracket
        | Scanned::CloseBracket
        | Scanned::Comma
        | Scanned::Colon
        | Scanned::Word(_)
        | Scanned::Boolean(_)
        | Scanned::Number(_)
        | Scanned::Null
        | Scanned::CommentLine(_)
        | Scanned::CommentBlock(_) => {
            languages::lossless_token(&mut writing.tree, kind, start, end, &mut gap);
        }
    }
}

/// Writes the gap to the end of the text.
fn finish(writing: &mut Writing<'_, '_, '_>, length: Offset) {
    let (text, reporting) = (writing.text, writing.reporting);
    languages::fill_to(
        &mut writing.tree,
        length,
        &mut |builder: &mut languages::TableBuilder<'_>, from, to| {
            unread(text, reporting, builder, from, to);
        },
    );
}

/// Writes the tokens that start before `before`, in the order of the file: the ones between the constructs.
fn drain(
    writing: &mut Writing<'_, '_, '_>,
    tokens: &[TokenAndRange<'_>],
    next: &mut Count,
    before: Offset,
) {
    while let Some(scanned) = tokens
        .get(next.0)
        .filter(|each| start_of(each.range) < before)
    {
        token(writing, scanned);
        next.0 += 1;
    }
}

/// Writes `value` of the library's tree: each object, member and array is opened where it starts and closed where it ends, and the
/// tokens the library scanned are written between.
fn walk(
    writing: &mut Writing<'_, '_, '_>,
    value: &Value<'_>,
    tokens: &[TokenAndRange<'_>],
    next: &mut Count,
) {
    match value {
        Value::Object(object) => {
            drain(writing, tokens, next, start_of(object.range));
            enter(writing, Piece::Object, start_of(object.range));
            for property in &object.properties {
                drain(writing, tokens, next, start_of(property.range));
                enter(writing, Piece::Member, start_of(property.range));
                walk(writing, &property.value, tokens, next);
                drain(writing, tokens, next, end_of(property.range));
                leave(writing, end_of(property.range));
            }
            drain(writing, tokens, next, end_of(object.range));
            leave(writing, end_of(object.range));
        }
        Value::Array(array) => {
            drain(writing, tokens, next, start_of(array.range));
            enter(writing, Piece::Array, start_of(array.range));
            for element in &array.elements {
                walk(writing, element, tokens, next);
            }
            drain(writing, tokens, next, end_of(array.range));
            leave(writing, end_of(array.range));
        }
        Value::StringLit(_)
        | Value::NumberLit(_)
        | Value::BooleanLit(_)
        | Value::NullKeyword(_) => {}
    }
}

/// Writes the tokens of a file the library read, opening each construct where it starts and closing it where it ends.
fn structured(writing: &mut Writing<'_, '_, '_>, parsed: &ParseResult<'_>) {
    let tokens = parsed.tokens.as_deref().unwrap_or_default();
    let mut next = Count(0);
    if let Some(value) = &parsed.value {
        walk(writing, value, tokens, &mut next);
    }
    drain(writing, tokens, &mut next, Offset(u32::MAX));
}

/// Writes the tokens the library scans before it fails, flat under the root.
fn flat(writer: &mut Writing<'_, '_, '_>) {
    let mut scanner = Scanner::new(writer.text.0, &scanning());
    while let Ok(Some(found)) = scanner.scan() {
        let piece = TokenAndRange {
            range: Range::new(scanner.token_start(), scanner.token_end()),
            token: found,
        };
        token(writer, &piece);
    }
}

/// What a file read as json as the standard defines it has that json with comments has not, as diagnostics: a comment, a comma
/// before a closing brace or bracket, and no value.
fn strictness(parsed: &ParseResult<'_>, builder: &mut languages::TableBuilder<'_>) {
    if parsed.value.is_none() {
        languages::diagnose(
            builder,
            languages::ReportText(NO_VALUE),
            Offset(0),
            Offset(0),
        );
    }
    let mut comma: Option<Range> = None;
    for scanned in parsed.tokens.iter().flatten() {
        match scanned.token {
            Scanned::CommentLine(_) | Scanned::CommentBlock(_) => languages::diagnose(
                builder,
                languages::ReportText(COMMENT_IN_JSON),
                start_of(scanned.range),
                end_of(scanned.range),
            ),
            Scanned::Comma => comma = Some(scanned.range),
            Scanned::CloseBrace | Scanned::CloseBracket => {
                if let Some(trailing) = comma.take() {
                    languages::diagnose(
                        builder,
                        languages::ReportText(TRAILING_COMMA_IN_JSON),
                        start_of(trailing),
                        end_of(trailing),
                    );
                }
            }
            Scanned::OpenBrace
            | Scanned::OpenBracket
            | Scanned::Colon
            | Scanned::String(_)
            | Scanned::Word(_)
            | Scanned::Boolean(_)
            | Scanned::Number(_)
            | Scanned::Null => comma = None,
        }
    }
}

/// What a string of the file has that the standard does not allow and the library reads: a control character written as it is. Both
/// dialects are told, as the configuration files of TypeScript and VS Code are.
fn controls(
    parsed: &ParseResult<'_>,
    text: JsoncText<'_>,
    builder: &mut languages::TableBuilder<'_>,
) {
    for scanned in parsed.tokens.iter().flatten() {
        if !matches!(scanned.token, Scanned::String(_)) {
            continue;
        }
        let inside = text
            .0
            .get(scanned.range.start..scanned.range.end)
            .unwrap_or_default();
        if let Some(found) = inside.bytes().position(|byte| byte < b' ') {
            let at = offset(Count(scanned.range.start + found));
            languages::diagnose(
                builder,
                languages::ReportText(CONTROL_IN_STRING),
                at,
                offset(Count(scanned.range.start + found + 1)),
            );
        }
    }
}

/// What the library said of a file it could not read, as a diagnostic.
fn refusal(error: &ParseError, builder: &mut languages::TableBuilder<'_>) {
    let message = error.to_string();
    let range = error.range();
    languages::diagnose(
        builder,
        languages::ReportText(&message),
        start_of(range),
        end_of(range),
    );
}

impl SyntaxPort for SyntaxJsoncParserAdapter {
    fn read(
        &self,
        text: SourceText<'_>,
        dialect: Dialect,
        builder: &mut languages::TableBuilder<'_>,
    ) {
        // JSON is UTF-8: the library reads the text up to the first byte that is not, and the rest is text it could not read.
        let content = languages::readable_text(ByteSlice(text.0)).0;
        let length = offset(Count(text.0.len()));
        languages::enter_node(builder, kind_bits(piece_kind(Piece::File)), Offset(0));
        let collect = CollectOptions {
            comments: CommentCollectionStrategy::AsTokens,
            tokens: true,
        };
        let outcome = parse_to_ast(content, &collect, &grammar());
        let mut writer = Writing {
            tree: languages::lossless(builder, Offset(0)),
            text: JsoncText(content),
            reporting: if outcome.is_ok() {
                Reporting::Told
            } else {
                Reporting::Silent
            },
        };
        match &outcome {
            Ok(parsed) => structured(&mut writer, parsed),
            Err(_) => flat(&mut writer),
        }
        finish(&mut writer, length);
        languages::leave_node(writer.tree.builder, length);
        if content.len() < text.0.len() {
            languages::diagnose(
                writer.tree.builder,
                languages::ReportText(NOT_UTF8),
                offset(Count(content.len())),
                length,
            );
        }
        match &outcome {
            Ok(parsed) => {
                controls(parsed, writer.text, writer.tree.builder);
                if dialect == Dialect::Strict {
                    strictness(parsed, writer.tree.builder);
                }
            }
            Err(error) => refusal(error, writer.tree.builder),
        }
    }
}
