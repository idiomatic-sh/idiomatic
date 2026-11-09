//! Certify lexical facts from the parser's consumed token stream and lexer state errors.
use super::constants::UNREADABLE;
use super::types::{
    ByteIndex, Decoded, DelimiterEof, LexicallyComplete, Readable, Strings, UnclosedDelimiter,
};
use crate::outbound::syntax::types::SourceText;
use ruff_python_ast::Mod;
use ruff_python_ast::token::{Token, TokenKind};
use ruff_python_ast::{AnyNodeRef, NodeKind as RuffNode};
use ruff_python_parser::{LexicalErrorType, ParseErrorType, Parsed};
use ruff_text_size::Ranged as _;
use ryo::languages::{self, Offset};
use std::borrow::Cow;

/// Parser recovery does not by itself make comment or indentation facts incomplete.
/// The parser consumes its token source through EOF. An unclosed delimiter reports
/// a synthetic zero-width lexer error there, after all source tokens were read.
/// Other lexer failures can lose string, interpolation or continuation state and
/// remain incomplete even when their token ranges happen to cover every byte.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only lexical and f-string or t-string errors bear on lexing; every other parse error is syntax over tokens the lexer completed"
)]
pub(super) fn complete(
    parsed: &Parsed<Mod>,
    source: languages::ByteSlice<'_>,
) -> LexicallyComplete {
    let mut delimiter_eof = DelimiterEof(false);
    for error in parsed.errors() {
        match &error.error {
            ParseErrorType::Lexical(LexicalErrorType::Eof)
                if error.location.is_empty()
                    && error.location.end().to_usize() == source.0.len() =>
            {
                delimiter_eof = DelimiterEof(true);
            }
            ParseErrorType::Lexical(_)
            | ParseErrorType::FStringError(_)
            | ParseErrorType::TStringError(_) => return LexicallyComplete(false),
            _ => {}
        }
    }
    if delimiter_eof.0 && !has_unclosed_delimiter(parsed.tokens()).0 {
        return LexicallyComplete(false);
    }
    covered(parsed.tokens(), source, delimiter_eof)
}

/// A delimiter witness distinguishes the EOF marker from failed continuations.
/// Mismatched delimiters do not certify the lexer's nesting state.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only brackets open or close a delimiter; every other token leaves the nesting as it is"
)]
fn has_unclosed_delimiter(tokens: &[Token]) -> UnclosedDelimiter {
    let mut expected = Vec::new();
    for token in tokens {
        match token.kind() {
            TokenKind::Lpar => expected.push(TokenKind::Rpar),
            TokenKind::Lsqb => expected.push(TokenKind::Rsqb),
            TokenKind::Lbrace => expected.push(TokenKind::Rbrace),
            closing @ (TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace) => {
                if expected.pop() != Some(closing) {
                    return UnclosedDelimiter(false);
                }
            }
            _ => {}
        }
    }
    UnclosedDelimiter(!expected.is_empty())
}

/// Ordered, bounded tokens must cover every non-whitespace source byte.
/// Only a separately certified synthetic EOF error token may be ignored.
pub(super) fn covered(
    tokens: &[Token],
    source: languages::ByteSlice<'_>,
    delimiter_eof: DelimiterEof,
) -> LexicallyComplete {
    if u32::try_from(source.0.len()).is_err() {
        return LexicallyComplete(false);
    }
    let mut previous_end = 0_u32;
    for (index, token) in tokens.iter().enumerate() {
        let start = token.start().to_u32();
        let end = token.end().to_u32();
        let start_index = usize::try_from(start).unwrap_or(usize::MAX);
        let end_index = usize::try_from(end).unwrap_or(usize::MAX);
        if start < previous_end
            || start > end
            || end_index > source.0.len()
            || !source
                .0
                .get(usize::try_from(previous_end).unwrap_or(usize::MAX)..start_index)
                .is_some_and(|gap| gap.iter().all(u8::is_ascii_whitespace))
        {
            return LexicallyComplete(false);
        }
        let kind = super::schemas::token_kind(token.kind());
        if kind
            == crate::domain::syntax::types::Kind::Token(
                crate::domain::tokens::types::Token::Errortoken,
            )
            && !(delimiter_eof.0
                && token.kind() == TokenKind::Unknown
                && start == end
                && end_index == source.0.len())
        {
            return LexicallyComplete(false);
        }
        if token.kind() == TokenKind::EndOfFile
            && (index + 1 != tokens.len() || end_index != source.0.len())
        {
            return LexicallyComplete(false);
        }
        previous_end = end;
    }
    LexicallyComplete(
        source
            .0
            .get(usize::try_from(previous_end).unwrap_or(usize::MAX)..)
            .is_some_and(|gap| gap.iter().all(u8::is_ascii_whitespace)),
    )
}

/// The text the library reads for `bytes`: the bytes themselves when they are UTF-8, or else each byte that is not part of a
/// character replaced by one the lexer cannot read, so every offset is the file's.
pub(super) fn readable(bytes: SourceText<'_>) -> Readable<'_> {
    if let Ok(text) = std::str::from_utf8(bytes.0) {
        return Readable(Cow::Borrowed(text));
    }
    let mut text = String::with_capacity(bytes.0.len());
    for chunk in bytes.0.utf8_chunks() {
        text.push_str(chunk.valid());
        text.extend(chunk.invalid().iter().map(|_| UNREADABLE));
    }
    Readable(Cow::Owned(text))
}

/// The text the library decoded for the tokens of a node: a string's value, or the literal part of an f-string's or a t-string's. Only
/// these two kinds of the library's nodes carry it; every other kind carries none.
pub(super) fn decoded_of(node: AnyNodeRef<'_>) -> Option<Decoded<'_>> {
    if let AnyNodeRef::StringLiteral(literal) = node {
        return Some(Decoded(&literal.value));
    }
    if let AnyNodeRef::InterpolatedStringLiteralElement(element) = node {
        return Some(Decoded(&element.value));
    }
    None
}

/// The offset of a byte index of the text.
pub(super) fn offset(index: ByteIndex) -> Offset {
    Offset(u32::try_from(index.0).unwrap_or(u32::MAX))
}

/// The kind of string a node of the library opens, for the interpolations inside it: an f-string or a format spec, a t-string, or
/// none. The standard's format spec is a `JoinedStr` in a t-string too: what it interpolates is a `FormattedValue`.
#[must_use]
pub(super) const fn opens(kind: RuffNode) -> Option<Strings> {
    if matches!(kind, RuffNode::ExprTString) {
        Some(Strings::Template)
    } else if matches!(
        kind,
        RuffNode::ExprFString | RuffNode::InterpolatedStringFormatSpec
    ) {
        Some(Strings::Formatted)
    } else {
        None
    }
}
