//! A minimal adapter of Ruff 0.16.5's `ruff_python_literal::cformat` parser.
//!
//! The upstream grammar is adapted from `crates/ruff_python_literal/src/cformat.rs` in the
//! Ruff 0.16.5 source tree (MIT license, Astral Software Inc. and contributors). This adapter
//! keeps only decoded string parsing and the fields needed to describe argument consumption;
//! parser-library types and lint policy do not cross this boundary.

use crate::domain::format_syntax::interface::FormatSyntax;
use crate::domain::format_syntax::service::insert_flag;
use crate::domain::format_syntax::types::{
    FormatArgument, FormatDialect, FormatErrorKind, FormatFieldValue, FormatFlags,
    FormatParseError, FormatParseResult, FormatQuantityMode, FormatSummary, ParsedFormat,
};

/// The parser used by the Python language composition.
pub(crate) struct RuffLiteralCFormat;

#[derive(Clone, Copy)]
/// One Unicode scalar and its position in the decoded string.
struct Character {
    /// Zero-based Unicode scalar index.
    index: usize,
    /// The decoded scalar.
    value: char,
}

/// A scalar iterator with one character of lookahead.
struct Cursor<'a> {
    /// Remaining scalar positions and values.
    characters: std::iter::Enumerate<std::str::Chars<'a>>,
    /// The lookahead value, if already read.
    peeked: Option<Character>,
}

impl<'a> Cursor<'a> {
    /// Begin reading one decoded input.
    fn new(input: &'a str) -> Self {
        Self {
            characters: input.chars().enumerate(),
            peeked: None,
        }
    }

    /// Borrow the next scalar without consuming it.
    fn peek(&mut self) -> Option<Character> {
        if self.peeked.is_none() {
            self.peeked = self
                .characters
                .next()
                .map(|(index, value)| Character { index, value });
        }
        self.peeked
    }

    /// Consume the next scalar, including any retained lookahead.
    fn next(&mut self) -> Option<Character> {
        self.peeked.take().or_else(|| {
            self.characters
                .next()
                .map(|(index, value)| Character { index, value })
        })
    }
}

/// The grammar fields retained from one conversion specification.
struct ParsedField {
    /// Argument consumption and conversion metadata.
    field: FormatFieldValue,
}

/// Syntax failure or inability to retain a complete result.
enum Failure {
    /// A concrete grammar diagnostic.
    Malformed(FormatParseError),
    /// The input could not be retained completely.
    Unavailable,
}

/// Construct a grammar diagnostic with a scalar-based index.
fn error(kind: FormatErrorKind, index: usize, character: Option<char>) -> FormatParseError {
    FormatParseError {
        kind,
        character,
        index,
    }
}

/// Consume an optional mapping key, respecting nested parentheses.
fn mapping_key(cursor: &mut Cursor<'_>) -> Result<Option<String>, Failure> {
    let Some(open) = cursor.peek() else {
        return Ok(None);
    };
    if open.value != '(' {
        return Ok(None);
    }
    let _open = cursor.next();
    let mut depth = 1_usize;
    let mut key = String::new();
    while let Some(character) = cursor.next() {
        match character.value {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return Ok(Some(key));
        }
        if key.try_reserve(character.value.len_utf8()).is_err() {
            return Err(Failure::Unavailable);
        }
        key.push(character.value);
    }
    Err(Failure::Malformed(error(
        FormatErrorKind::UnmatchedKeyParentheses,
        open.index,
        None,
    )))
}

/// Consume optional constant or star width and precision syntax.
fn quantity(cursor: &mut Cursor<'_>) -> Result<FormatQuantityMode, Failure> {
    match cursor.peek() {
        Some(character) if character.value == '*' => {
            let _star = cursor.next();
            Ok(FormatQuantityMode::Star)
        }
        Some(first) if first.value.is_ascii_digit() => {
            let mut amount = 0_i32;
            while let Some(character) = cursor.peek().filter(|each| each.value.is_ascii_digit()) {
                let digit = character.value.to_digit(10).ok_or(Failure::Unavailable)?;
                amount = amount
                    .checked_mul(10)
                    .and_then(|number| number.checked_add(i32::try_from(digit).ok()?))
                    .ok_or_else(|| {
                        Failure::Malformed(error(
                            FormatErrorKind::WidthOrPrecisionTooLarge,
                            character.index,
                            None,
                        ))
                    })?;
                let _digit = cursor.next();
            }
            Ok(FormatQuantityMode::Fixed)
        }
        Some(_) | None => Ok(FormatQuantityMode::None),
    }
}

/// Consume one complete conversion specification after the percent sign.
fn parse_field(cursor: &mut Cursor<'_>, ordinal: usize) -> Result<ParsedField, Failure> {
    let key = mapping_key(cursor)?;

    let mut flags = FormatFlags::default();
    while cursor
        .peek()
        .is_some_and(|character| insert_flag(&mut flags, character.value))
    {
        let _flag = cursor.next();
    }

    let width = quantity(cursor)?;
    let precision = if cursor
        .peek()
        .is_some_and(|character| character.value == '.')
    {
        let _dot = cursor.next();
        match quantity(cursor)? {
            FormatQuantityMode::None => FormatQuantityMode::Dot,
            mode @ (FormatQuantityMode::Fixed
            | FormatQuantityMode::Star
            | FormatQuantityMode::Dot) => mode,
        }
    } else {
        FormatQuantityMode::None
    };

    if cursor
        .peek()
        .is_some_and(|character| matches!(character.value, 'h' | 'l' | 'L'))
    {
        let _length = cursor.next();
    }

    let Some(conversion) = cursor.next() else {
        let index = cursor.peek().map_or(0, |character| character.index);
        return Err(Failure::Malformed(error(
            FormatErrorKind::IncompleteFormat,
            index,
            None,
        )));
    };
    if !matches!(
        conversion.value,
        'd' | 'i'
            | 'u'
            | 'o'
            | 'x'
            | 'X'
            | 'e'
            | 'E'
            | 'f'
            | 'F'
            | 'g'
            | 'G'
            | 'c'
            | 'r'
            | 's'
            | 'a'
    ) {
        return Err(Failure::Malformed(error(
            FormatErrorKind::UnsupportedFormatCharacter,
            conversion.index,
            Some(conversion.value),
        )));
    }

    Ok(ParsedField {
        field: FormatFieldValue {
            ordinal,
            argument: key.map_or(FormatArgument::Positional, FormatArgument::Mapping),
            conversion: conversion.value,
            width,
            precision,
            flags,
        },
    })
}

/// Parse all conversions and retain their ordered argument consumption summary.
fn parse_percent_string(input: &str) -> Result<ParsedFormat, Failure> {
    let mut cursor = Cursor::new(input);
    let mut fields = Vec::new();
    let mut summary = FormatSummary::default();
    while let Some(character) = cursor.next() {
        if character.value != '%' {
            continue;
        }
        let Some(next) = cursor.peek() else {
            return Err(Failure::Malformed(error(
                FormatErrorKind::IncompleteFormat,
                character.index + 1,
                None,
            )));
        };
        if next.value == '%' {
            let _escaped = cursor.next();
            continue;
        }
        let parsed = parse_field(&mut cursor, fields.len())?;
        if matches!(parsed.field.argument, FormatArgument::Mapping(_)) {
            summary.mapping_present = true;
        } else {
            summary.positional_consumption += 1;
        }
        if parsed.field.width == FormatQuantityMode::Star {
            summary.positional_consumption += 1;
            summary.dynamic_width = true;
        }
        if parsed.field.precision == FormatQuantityMode::Star {
            summary.positional_consumption += 1;
            summary.dynamic_precision = true;
        }
        if fields.try_reserve(1).is_err() {
            return Err(Failure::Unavailable);
        }
        fields.push(parsed.field);
    }
    Ok(ParsedFormat { fields, summary })
}

impl FormatSyntax for RuffLiteralCFormat {
    fn parse(input: &str, dialect: FormatDialect) -> FormatParseResult {
        match dialect {
            FormatDialect::PercentString => match parse_percent_string(input) {
                Ok(parsed) => FormatParseResult::Parsed(parsed),
                Err(Failure::Malformed(error)) => FormatParseResult::Malformed(error),
                Err(Failure::Unavailable) => FormatParseResult::Unavailable,
            },
        }
    }
}

#[cfg(test)]
#[path = "ruff_literal_cformat_test.rs"]
mod tests;
