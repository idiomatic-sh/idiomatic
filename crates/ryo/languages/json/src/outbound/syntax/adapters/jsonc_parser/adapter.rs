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
use jsonc_parser::{CollectOptions, CommentCollectionStrategy, ParseOptions, ParseResult, Scanner, ScannerOptions, parse_to_ast};
use ryo::languages::{self, ByteSlice, Count, Offset, node_table};

use crate::domain::syntax::service::{field_bits, kind_bits};
use crate::domain::syntax::types::{Dialect, Field};
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;

use super::constants::{COMMENT_IN_JSON, CONTROL_IN_STRING, NO_VALUE, NOT_UTF8, TRAILING_COMMA_IN_JSON};
use super::schemas::{piece_kind};
use super::interface::Sink;
use super::types::{JsoncText, Piece, Reporting};
use super::walk::Numbered;

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

/// Writes the tokens that start before `before`, in the order of the file: the ones between the constructs.
fn drain(
    writing: &mut impl Sink,
    tokens: &[TokenAndRange<'_>],
    next: &mut Count,
    before: Offset,
) {
    while let Some(scanned) = tokens
        .get(next.0)
        .filter(|each| start_of(each.range) < before)
    {
        writing.token(scanned);
        next.0 += 1;
    }
}

/// Writes `value` of the library's tree: each object, member and array is opened where it starts and closed where it ends, and the
/// tokens the library scanned are written between.
fn walk(
    writing: &mut impl Sink,
    value: &Value<'_>,
    tokens: &[TokenAndRange<'_>],
    next: &mut Count,
) {
    match value {
        Value::Object(object) => {
            drain(writing, tokens, next, start_of(object.range));
            writing.enter(Piece::Object, start_of(object.range));
            for property in &object.properties {
                drain(writing, tokens, next, start_of(property.range));
                writing.enter(Piece::Member, start_of(property.range));
                walk(writing, &property.value, tokens, next);
                drain(writing, tokens, next, end_of(property.range));
                writing.leave(end_of(property.range));
            }
            drain(writing, tokens, next, end_of(object.range));
            writing.leave(end_of(object.range));
        }
        Value::Array(array) => {
            drain(writing, tokens, next, start_of(array.range));
            writing.enter(Piece::Array, start_of(array.range));
            for element in &array.elements {
                walk(writing, element, tokens, next);
            }
            drain(writing, tokens, next, end_of(array.range));
            writing.leave(end_of(array.range));
        }
        Value::StringLit(_)
        | Value::NumberLit(_)
        | Value::BooleanLit(_)
        | Value::NullKeyword(_) => {}
    }
}

/// Writes the tokens of a file the library read, opening each construct where it starts and closing it where it ends.
fn structured(writing: &mut impl Sink, parsed: &ParseResult<'_>) {
    let tokens = parsed.tokens.as_deref().unwrap_or_default();
    let mut next = Count(0);
    if let Some(value) = &parsed.value {
        walk(writing, value, tokens, &mut next);
    }
    drain(writing, tokens, &mut next, Offset(u32::MAX));
}

/// Writes the tokens the library scans before it fails, flat under the root.
fn flat(writer: &mut impl Sink, text: JsoncText<'_>) {
    let mut scanner = Scanner::new(text.0, &scanning());
    while let Ok(Some(found)) = scanner.scan() {
        let piece = TokenAndRange {
            range: Range::new(scanner.token_start(), scanner.token_end()),
            token: found,
        };
        writer.token(&piece);
    }
}

/// What a file read as json as the standard defines it has that json with comments has not, as diagnostics: a comment, a comma
/// before a closing brace or bracket, and no value.
fn strictness(parsed: &ParseResult<'_>, builder: &mut impl Sink) {
    if parsed.value.is_none() {
        builder.diagnose(NO_VALUE, Offset(0), Offset(0),
        );
    }
    let mut comma: Option<Range> = None;
    for scanned in parsed.tokens.iter().flatten() {
        match scanned.token {
            Scanned::CommentLine(_) | Scanned::CommentBlock(_) => builder.diagnose(COMMENT_IN_JSON, start_of(scanned.range), end_of(scanned.range),
            ),
            Scanned::Comma => comma = Some(scanned.range),
            Scanned::CloseBrace | Scanned::CloseBracket => {
                if let Some(trailing) = comma.take() {
                    builder.diagnose(TRAILING_COMMA_IN_JSON, start_of(trailing), end_of(trailing),
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
    builder: &mut impl Sink,
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
            builder.diagnose(CONTROL_IN_STRING, at, offset(Count(scanned.range.start + found + 1)),
            );
        }
    }
}

/// What the library said of a file it could not read, as a diagnostic.
fn refusal(error: &ParseError, builder: &mut impl Sink) {
    let message = error.to_string();
    let range = error.range();
    builder.diagnose(&message, start_of(range), end_of(range),
    );
}

impl SyntaxPort for SyntaxJsoncParserAdapter {

    fn walk(
        &self,
        text: SourceText<'_>,
        dialect: Dialect,
        decodings: &mut crate::domain::file_view::types::Decodings,
        builder: &mut languages::NodeTableBuilder<'_>,
    ) {
        let content = languages::readable_text(ByteSlice(text.0)).0;
        node_table::enter_node(
            builder,
            kind_bits(piece_kind(Piece::File)),
            languages::FieldCode(field_bits(Field::None).0),
            Offset(0),
        );
        let outcome = parsed(content);
        let mut writer = Numbered::new(builder, decodings, JsoncText(content), reporting_of(&outcome));
        write(&mut writer, &outcome, content, text.0.len(), dialect);
        node_table::set_syntax_acceptance(
            writer.builder,
            // the file is accepted when nothing was said of it
            if writer.said == 0 {
                languages::SyntaxAcceptance::Accepted
            } else {
                languages::SyntaxAcceptance::Rejected
            },
        );
    }
}

/// What the library makes of `content`.
fn parsed(content: &str) -> Result<ParseResult<'_>, ParseError> {
    let collect = CollectOptions {
        comments: CommentCollectionStrategy::AsTokens,
        tokens: true,
    };
    parse_to_ast(content, &collect, &grammar())
}

/// Whether text no token spans is told: when the library read the file it said nothing of it, and when it did not, it said what it
/// could.
fn reporting_of(outcome: &Result<ParseResult<'_>, ParseError>) -> Reporting {
    if outcome.is_ok() {
        Reporting::Told
    } else {
        Reporting::Silent
    }
}

/// Writes what the library read of a file into `writer`, which holds the file node open, then what was said of the file.
fn write<S: Sink>(
    writer: &mut S,
    outcome: &Result<ParseResult<'_>, ParseError>,
    content: &str,
    whole: usize,
    dialect: Dialect,
) {
    let length = offset(Count(whole));
    match outcome {
        Ok(parsed) => structured(writer, parsed),
        Err(_) => flat(writer, JsoncText(content)),
    }
    writer.close(length);
    if content.len() < whole {
        writer.diagnose(NOT_UTF8, offset(Count(content.len())), length);
    }
    match outcome {
        Ok(parsed) => {
            controls(parsed, JsoncText(content), writer);
            if dialect == Dialect::Strict {
                strictness(parsed, writer);
            }
        }
        Err(error) => refusal(error, writer),
    }
}
