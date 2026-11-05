//! The `literals` service: free functions named by verb over the domain's types, with no IO and no ports. It decodes a string literal
//! as `strconv.Unquote` does; a literal that is no text (an escape that makes a byte that is no UTF-8) decodes to nothing.

use std::borrow::Cow;

use super::constants::{
    BACK_QUOTE, ESCAPE, HEX_DIGITS, HEXADECIMAL, LONG_UNICODE_DIGITS, NOTHING, OCTAL, OCTAL_DIGITS,
    QUOTE, RETURN, SHORT_UNICODE_DIGITS,
};
use super::types::{
    Code, Delimiter, Digits, Escaped, Glyph, Numbered, Octet, Radix, Unquoted, Written,
};

/// The number the next `digits` digits of `base` of `rest` write, and what follows them.
fn number(rest: Written<'_>, digits: Digits, base: Radix) -> Option<Numbered<'_>> {
    let mut value = 0_u32;
    let mut chars = rest.0.char_indices();
    for _ in 0..digits.0 {
        let (_, digit) = chars.next()?;
        value = value
            .checked_mul(base.0)?
            .checked_add(digit.to_digit(base.0)?)?;
    }
    let after = chars.next().map_or(rest.0.len(), |(at, _)| at);
    Some(Numbered {
        code: Code(value),
        after: Written(rest.0.get(after..)?),
    })
}

/// The escape that opens `rest`, which follows a backslash.
fn escape(rest: Written<'_>) -> Option<Escaped<'_>> {
    let mut chars = rest.0.chars();
    let first = chars.next()?;
    let after = Written(chars.as_str());
    let glyph = match first {
        'a' => '\u{7}',
        'b' => '\u{8}',
        'f' => '\u{c}',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        'v' => '\u{b}',
        '\\' => '\\',
        '"' => '"',
        'x' => {
            let read = number(after, HEX_DIGITS, HEXADECIMAL)?;
            return Some(Escaped::Byte {
                octet: Octet(u8::try_from(read.code.0).ok()?), // an escape past 255 is no byte: the Go specification refuses it, and the literal has no value
                after: read.after,
            });
        }
        'u' | 'U' => {
            let digits = if first == 'u' {
                SHORT_UNICODE_DIGITS
            } else {
                LONG_UNICODE_DIGITS
            };
            let read = number(after, digits, HEXADECIMAL)?;
            return Some(Escaped::Character {
                glyph: Glyph(char::from_u32(read.code.0)?),
                after: read.after,
            });
        }
        '0'..='7' => {
            let read = number(rest, OCTAL_DIGITS, OCTAL)?;
            return Some(Escaped::Byte {
                octet: Octet(u8::try_from(read.code.0).ok()?), // an escape past 255 is no byte: the Go specification refuses it, and the literal has no value
                after: read.after,
            });
        }
        _ => return None,
    };
    Some(Escaped::Character {
        glyph: Glyph(glyph),
        after,
    })
}

/// The text of the interpreted string whose content is `content`, between its quotes.
fn interpreted(content: Written<'_>) -> Option<Unquoted<'_>> {
    if !content.0.contains(ESCAPE) {
        return Some(Unquoted(Cow::Borrowed(content.0)));
    }
    let mut bytes = Vec::with_capacity(content.0.len());
    let mut rest = content.0;
    while let Some(at) = rest.find(ESCAPE) {
        bytes.extend_from_slice(rest.get(..at)?.as_bytes());
        match escape(Written(rest.get(at + ESCAPE.len_utf8()..)?))? {
            Escaped::Character { glyph, after } => {
                let mut buffer = [0; 4];
                bytes.extend_from_slice(glyph.0.encode_utf8(&mut buffer).as_bytes());
                rest = after.0;
            }
            Escaped::Byte { octet, after } => {
                bytes.push(octet.0);
                rest = after.0;
            }
        }
    }
    bytes.extend_from_slice(rest.as_bytes());
    String::from_utf8(bytes)
        .ok() // escapes that spell bytes which are no text leave the literal with no text value
        .map(|text| Unquoted(Cow::Owned(text)))
}

/// The text of the raw string whose content is `content`, between its back quotes: Go drops its carriage returns.
fn raw(content: Written<'_>) -> Unquoted<'_> {
    Unquoted(if content.0.contains(RETURN) {
        Cow::Owned(content.0.replace(RETURN, NOTHING))
    } else {
        Cow::Borrowed(content.0)
    })
}

/// What the literal `literal` holds between `delimiter` and `delimiter`.
fn between(literal: Written<'_>, delimiter: Delimiter) -> Option<Written<'_>> {
    literal
        .0
        .strip_prefix(delimiter.0)?
        .strip_suffix(delimiter.0)
        .map(Written)
}

/// The text the string literal `literal` stands for; nothing for a literal that is no text.
#[must_use]
pub(crate) fn unquoted(literal: Written<'_>) -> Option<Unquoted<'_>> {
    if let Some(content) = between(literal, QUOTE) {
        interpreted(content)
    } else {
        between(literal, BACK_QUOTE).map(raw)
    }
}
