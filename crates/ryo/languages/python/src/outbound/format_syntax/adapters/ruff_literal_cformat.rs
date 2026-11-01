//! A minimal adapter of Ruff 0.16.5's `ruff_python_literal::cformat` parser.
//!
//! The upstream grammar is adapted from `crates/ruff_python_literal/src/cformat.rs` in the
//! Ruff 0.16.5 source tree (MIT license, Astral Software Inc. and contributors). This adapter
//! keeps only decoded string parsing and the fields needed to describe argument consumption;
//! parser-library types and lint policy do not cross this boundary.

use crate::domain::format_syntax::interface::FormatSyntax;
use crate::domain::format_syntax::types::{
    FormatArgument, FormatDialect, FormatErrorKind, FormatFieldValue, FormatParseError,
    FormatParseResult, FormatQuantityMode, FormatSummary, ParsedFormat,
};

/// The parser used by the Python language composition.
pub(crate) struct RuffLiteralCFormat;

#[derive(Clone, Copy)]
struct Character {
    index: usize,
    value: char,
}

struct Cursor<'a> {
    characters: std::iter::Enumerate<std::str::Chars<'a>>,
    peeked: Option<Character>,
}

impl<'a> Cursor<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            characters: input.chars().enumerate(),
            peeked: None,
        }
    }

    fn peek(&mut self) -> Option<Character> {
        if self.peeked.is_none() {
            self.peeked = self
                .characters
                .next()
                .map(|(index, value)| Character { index, value });
        }
        self.peeked
    }

    fn next(&mut self) -> Option<Character> {
        self.peeked.take().or_else(|| {
            self.characters
                .next()
                .map(|(index, value)| Character { index, value })
        })
    }
}

struct ParsedField {
    field: FormatFieldValue,
}

enum Failure {
    Malformed(FormatParseError),
    Unavailable,
}

fn error(kind: FormatErrorKind, index: usize, character: Option<char>) -> FormatParseError {
    FormatParseError {
        kind,
        character,
        index,
    }
}

fn mapping_key(cursor: &mut Cursor<'_>) -> Result<Option<String>, Failure> {
    let Some(open) = cursor.peek() else {
        return Ok(None);
    };
    if open.value != '(' {
        return Ok(None);
    }
    let _open = cursor.next();
    let mut depth = 1usize;
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

fn quantity(cursor: &mut Cursor<'_>) -> Result<FormatQuantityMode, Failure> {
    match cursor.peek() {
        Some(character) if character.value == '*' => {
            let _star = cursor.next();
            Ok(FormatQuantityMode::Star)
        }
        Some(character) if character.value.is_ascii_digit() => {
            let mut amount = 0i32;
            while let Some(character) = cursor.peek().filter(|each| each.value.is_ascii_digit()) {
                let digit = character
                    .value
                    .to_digit(10)
                    .ok_or_else(|| Failure::Unavailable)?;
                amount = amount
                    .checked_mul(10)
                    .and_then(|number| {
                        i32::try_from(digit)
                            .ok()
                            .and_then(|digit| number.checked_add(digit))
                    })
                    .ok_or_else(|| {
                        Failure::Malformed(error(
                            FormatErrorKind::WidthOrPrecisionTooLarge,
                            character.index,
                            None,
                        ))
                    })?;
                let _digit = cursor.next();
            }
            let _amount = amount;
            Ok(FormatQuantityMode::Fixed)
        }
        Some(_) | None => Ok(FormatQuantityMode::None),
    }
}

fn parse_field(cursor: &mut Cursor<'_>, ordinal: usize) -> Result<ParsedField, Failure> {
    let key = mapping_key(cursor)?;

    while cursor
        .peek()
        .is_some_and(|character| matches!(character.value, '#' | '0' | '-' | ' ' | '+'))
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
        },
    })
}

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
mod tests {
    use super::*;

    fn parsed(source: &str) -> ParsedFormat {
        match RuffLiteralCFormat::parse(source, FormatDialect::PercentString) {
            FormatParseResult::Parsed(parsed) => parsed,
            FormatParseResult::Malformed(_) | FormatParseResult::Unavailable => {
                panic!("expected parsed format for {source:?}, got a non-parsed result")
            }
        }
    }

    #[test]
    fn escaped_percent_is_a_valid_zero_field_format() {
        let result = parsed("100%%");
        assert!(result.fields.is_empty());
        assert_eq!(result.summary, FormatSummary::default());
    }

    #[test]
    fn dynamic_width_and_precision_each_consume_positional_values() {
        let result = parsed("%*.*s");
        assert_eq!(result.summary.positional_consumption, 3);
        assert!(result.summary.dynamic_width);
        assert!(result.summary.dynamic_precision);
        assert_eq!(result.fields[0].width, FormatQuantityMode::Star);
        assert_eq!(result.fields[0].precision, FormatQuantityMode::Star);
    }

    #[test]
    fn repeated_mapping_keys_and_mixed_positional_fields_are_preserved() {
        let result = parsed("%(name)s %(name)r %s");
        assert_eq!(result.fields.len(), 3);
        assert_eq!(
            result.fields[0].argument,
            FormatArgument::Mapping("name".to_owned())
        );
        assert_eq!(
            result.fields[1].argument,
            FormatArgument::Mapping("name".to_owned())
        );
        assert_eq!(result.fields[2].argument, FormatArgument::Positional);
        assert_eq!(result.summary.positional_consumption, 1);
        assert!(result.summary.mapping_present);
    }

    #[test]
    fn malformed_percent_and_unicode_error_indices_use_scalar_offsets() {
        assert_eq!(
            RuffLiteralCFormat::parse("é%", FormatDialect::PercentString),
            FormatParseResult::Malformed(FormatParseError {
                kind: FormatErrorKind::IncompleteFormat,
                character: None,
                index: 2,
            }),
        );
        assert_eq!(
            RuffLiteralCFormat::parse("é%n", FormatDialect::PercentString),
            FormatParseResult::Malformed(FormatParseError {
                kind: FormatErrorKind::UnsupportedFormatCharacter,
                character: Some('n'),
                index: 2,
            }),
        );
    }

    #[test]
    fn unmatched_mapping_and_overflow_are_diagnostics() {
        assert!(matches!(
            RuffLiteralCFormat::parse("%(name", FormatDialect::PercentString),
            FormatParseResult::Malformed(FormatParseError {
                kind: FormatErrorKind::UnmatchedKeyParentheses,
                ..
            })
        ));
        assert!(matches!(
            RuffLiteralCFormat::parse("%999999999999999999999s", FormatDialect::PercentString),
            FormatParseResult::Malformed(FormatParseError {
                kind: FormatErrorKind::WidthOrPrecisionTooLarge,
                ..
            })
        ));
        assert_eq!(
            RuffLiteralCFormat::parse("%*", FormatDialect::PercentString),
            FormatParseResult::Malformed(FormatParseError {
                kind: FormatErrorKind::IncompleteFormat,
                character: None,
                index: 0,
            }),
        );
        assert!(matches!(
            RuffLiteralCFormat::parse("%b", FormatDialect::PercentString),
            FormatParseResult::Malformed(FormatParseError {
                kind: FormatErrorKind::UnsupportedFormatCharacter,
                character: Some('b'),
                ..
            })
        ));
    }
}
