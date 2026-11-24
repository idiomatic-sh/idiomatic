//! The adapter's writing of the parser's steps into the element table: a node entered and left, a token with the value it stands
//! for, and what the parser could not read. A string token carries the value its escapes resolve to; a numeric literal node carries
//! the kind, value and suffix the language reads from its token.

use super::constants::{CRLF, LF};
use super::schemas::kind_of;
use super::types::{Cursor, DecodedString, RustText, Step, StringLiteral};
use crate::domain::numeric_literals::constants::{
    FLOAT_NUMBER, INTEGER_NUMBER, NUMERIC_KIND, NUMERIC_SUFFIX, NUMERIC_VALUE,
};
use crate::domain::numeric_literals::types::NumberValue;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::TokenText;
use ra_ap_syntax::SyntaxKind;
use rustc_literal_escaper::unescape_str;
use ryo::languages::{self, Flag, NameText, Offset};

pub(crate) trait Output {
    fn numeric_values(&self) -> bool { true }
    fn enter(&mut self, kind: languages::KindBits, start: Offset);
    fn leave(&mut self, end: Offset);
    fn token(&mut self, kind: languages::KindBits, start: Offset, end: Offset, decoded: Option<NameText<'_>>);
    fn property(&mut self, key: languages::PropertyId, value: NameText<'_>);
    fn formatted(&mut self, key: languages::PropertyId, value: std::fmt::Arguments<'_>) -> std::fmt::Result;
    fn diagnostic(&mut self, message: &str, start: Offset, end: Offset);
    fn adopt(&mut self, host: languages::Position);
    fn origin(&mut self, at: languages::Position, host: languages::Position, expansion: languages::ExpansionId);
    fn release(&mut self);
}

impl Output for languages::TableBuilder<'_> {
    fn enter(&mut self, kind: languages::KindBits, start: Offset) { languages::enter_node(self, kind, start); }
    fn leave(&mut self, end: Offset) { languages::leave_node(self, end); }
    fn token(&mut self, kind: languages::KindBits, start: Offset, end: Offset, decoded: Option<NameText<'_>>) {
        match decoded {
            Some(value) => languages::decoded_token(self, kind, start, end, value),
            None => languages::token(self, kind, start, end),
        }
    }
    fn property(&mut self, key: languages::PropertyId, value: NameText<'_>) { languages::node_property(self, key, value); }
    fn formatted(&mut self, key: languages::PropertyId, value: std::fmt::Arguments<'_>) -> std::fmt::Result {
        languages::node_property_fmt(self, key, value)
    }
    fn diagnostic(&mut self, message: &str, start: Offset, end: Offset) { languages::diagnose(self, languages::ReportText(message), start, end); }
    fn adopt(&mut self, host: languages::Position) { languages::adopt(self, host); }
    fn origin(&mut self, at: languages::Position, host: languages::Position, expansion: languages::ExpansionId) { languages::mark_expansion(self, at, host, expansion); }
    fn release(&mut self) { languages::release(self); }
}

/// The value a string literal stands for, with its escapes resolved; nothing where an escape is wrong. A raw string is its text between
/// its delimiters, a carriage return and a line feed in it being one line feed.
fn decoded(literal: StringLiteral<'_>) -> Option<DecodedString> {
    let literal = literal.0;
    if let Some(raw) = literal.strip_prefix('r') {
        let hashes = raw.len() - raw.trim_start_matches('#').len();
        let inner = raw.get(hashes + 1..raw.len().checked_sub(hashes + 1)?)?;
        return Some(DecodedString(inner.replace(CRLF, LF)));
    }
    let inner = literal.strip_prefix('"')?.strip_suffix('"')?;
    let mut value = String::with_capacity(inner.len());
    let mut wrong = false;
    unescape_str(inner, |_, character| match character {
        Ok(each) => value.push(each),
        Err(_) => wrong = true,
    });
    (!wrong).then_some(DecodedString(value))
}

/// Writes what the numeric literal `written` stands for as the properties of the literal node open around it: its kind, its value
/// (an integer's base-10 digits, a float's one binary64 spelling) and the suffix written after it. A literal a compiler refuses gets
/// none.
fn numeric(builder: &mut impl Output, written: TokenText<'_>) {
    let Some(literal) = crate::domain::numeric_literals::service::decoded(written) else {
        return;
    };
    match literal.value {
        NumberValue::Integer(value) => {
            builder.property(NUMERIC_KIND, NameText(INTEGER_NUMBER));
            let formatted =
                builder.formatted(NUMERIC_VALUE, format_args!("{}", value.0));
            debug_assert!(formatted.is_ok(), "an integer formats without failure");
        }
        NumberValue::Float(bits) => {
            builder.property(NUMERIC_KIND, NameText(FLOAT_NUMBER));
            let mut buffer = languages::FloatBuffer([0; languages::FLOAT_TEXT_BYTES]);
            let text = languages::binary64_text(bits, &mut buffer);
            builder.property(NUMERIC_VALUE, text);
        }
    }
    if let Some(suffix) = literal.suffix {
        builder.property(NUMERIC_SUFFIX, suffix);
    }
}

/// Writes `step` into the table at the cursor, which moves past the text of a token.
pub(super) fn emit(
    builder: &mut impl Output,
    text: RustText<'_>,
    step: &Step,
    cursor: &mut Cursor,
) {
    match step {
        Step::Enter(kind) => {
            cursor.next.0 += 1;
            cursor.literal = if *kind == SyntaxKind::LITERAL {
                Flag::Raised
            } else {
                Flag::Lowered
            };
            builder.enter(kind_bits(kind_of(*kind)), cursor.position);
        }
        Step::Token(kind, length) => {
            cursor.next.0 += 1;
            let end = Offset(
                cursor
                    .position
                    .0
                    .saturating_add(u32::try_from(length.0).unwrap_or(u32::MAX)),
            );
            let from = usize::try_from(cursor.position.0).unwrap_or(usize::MAX);
            let to = usize::try_from(end.0).unwrap_or(usize::MAX);
            let value = (*kind == SyntaxKind::STRING)
                .then(|| {
                    let written = text.0.get(from..to)?;
                    decoded(StringLiteral(written))
                })
                .flatten();
            builder.token(kind_bits(kind_of(*kind)), cursor.position, end, value.as_ref().map(|v| NameText(&v.0)));
            if cursor.literal == Flag::Raised
                && matches!(kind, SyntaxKind::INT_NUMBER | SyntaxKind::FLOAT_NUMBER)
                && builder.numeric_values()
                && let Some(written) = text.0.get(from..to)
            {
                numeric(builder, TokenText(written));
            }
            cursor.position = end;
        }
        Step::Exit => {
            cursor.literal = Flag::Lowered;
            builder.leave(cursor.position);
        }
        Step::Error(message, at) => {
            builder.diagnostic(message, *at, *at);
        }
    }
}
