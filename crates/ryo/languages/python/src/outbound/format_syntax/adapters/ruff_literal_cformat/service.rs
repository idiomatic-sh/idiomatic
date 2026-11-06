//! The cformat grammar: reading conversion specifications from a decoded string.

use super::types::{Character, Cursor, Failure, ParsedField};
use crate::domain::format_syntax::service::insert_flag;
use crate::domain::format_syntax::types::{
    FlagRecognition, FormatArgument, FormatCharacter, FormatErrorKind, FormatFieldOrdinal,
    FormatFieldValue, FormatFlags, FormatInput, FormatMappingKey, FormatParseError, FormatProperty,
    FormatQuantityMode, FormatScalarIndex, FormatSummary, ParsedFormat,
};

/// Begin reading one decoded input.
pub(super) fn cursor(input: FormatInput<'_>) -> Cursor<'_> {
    Cursor {
        characters: input.0.chars().enumerate(),
        peeked: None,
    }
}

/// Borrow the next scalar without consuming it.
pub(super) fn peek(cursor: &mut Cursor<'_>) -> Option<Character> {
    if cursor.peeked.is_none() {
        cursor.peeked = cursor.characters.next().map(|(index, value)| Character {
            index: FormatScalarIndex(index),
            value: FormatCharacter(value),
        });
    }
    cursor.peeked
}

/// Consume the next scalar, including any retained lookahead.
pub(super) fn advance(cursor: &mut Cursor<'_>) -> Option<Character> {
    cursor.peeked.take().or_else(|| {
        cursor.characters.next().map(|(index, value)| Character {
            index: FormatScalarIndex(index),
            value: FormatCharacter(value),
        })
    })
}

/// Construct a grammar diagnostic with a scalar-based index.
fn error(
    kind: FormatErrorKind,
    index: FormatScalarIndex,
    character: Option<FormatCharacter>,
) -> FormatParseError {
    FormatParseError {
        kind,
        character,
        index,
    }
}

/// Consume an optional mapping key, respecting nested parentheses.
fn mapping_key(cursor: &mut Cursor<'_>) -> Result<Option<FormatMappingKey>, Failure> {
    let Some(open) = peek(cursor) else {
        return Ok(None);
    };
    if open.value.0 != '(' {
        return Ok(None);
    }
    let _open = advance(cursor);
    let mut depth = 1_usize;
    let mut key = String::new();
    while let Some(character) = advance(cursor) {
        match character.value.0 {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return Ok(Some(FormatMappingKey(key)));
        }
        if key.try_reserve(character.value.0.len_utf8()).is_err() {
            return Err(Failure::Unavailable);
        }
        key.push(character.value.0);
    }
    Err(Failure::Malformed(error(
        FormatErrorKind::UnmatchedKeyParentheses,
        open.index,
        None,
    )))
}

/// Consume optional constant or star width and precision syntax.
fn quantity(cursor: &mut Cursor<'_>) -> Result<FormatQuantityMode, Failure> {
    match peek(cursor) {
        Some(character) if character.value.0 == '*' => {
            let _star = advance(cursor);
            Ok(FormatQuantityMode::Star)
        }
        Some(first) if first.value.0.is_ascii_digit() => {
            let mut amount = 0_i32;
            while let Some(character) = peek(cursor).filter(|each| each.value.0.is_ascii_digit()) {
                let digit = character.value.0.to_digit(10).ok_or(Failure::Unavailable)?;
                amount = amount
                    .checked_mul(10)
                    .and_then(|number| {
                        let Ok(digit) = i32::try_from(digit) else {
                            return None;
                        };
                        number.checked_add(digit)
                    })
                    .ok_or_else(|| {
                        Failure::Malformed(error(
                            FormatErrorKind::WidthOrPrecisionTooLarge,
                            character.index,
                            None,
                        ))
                    })?;
                let _digit = advance(cursor);
            }
            Ok(FormatQuantityMode::Fixed)
        }
        Some(_) | None => Ok(FormatQuantityMode::None),
    }
}

/// Consume one complete conversion specification after the percent sign.
fn parse_field(
    cursor: &mut Cursor<'_>,
    ordinal: FormatFieldOrdinal,
) -> Result<ParsedField, Failure> {
    let key = mapping_key(cursor)?;

    let mut flags = FormatFlags::default();
    while peek(cursor).is_some_and(|character| {
        insert_flag(&mut flags, character.value) == FlagRecognition::Consumed
    }) {
        let _flag = advance(cursor);
    }

    let width = quantity(cursor)?;
    let precision = if peek(cursor).is_some_and(|character| character.value.0 == '.') {
        let _dot = advance(cursor);
        match quantity(cursor)? {
            FormatQuantityMode::None => FormatQuantityMode::Dot,
            mode @ (FormatQuantityMode::Fixed
            | FormatQuantityMode::Star
            | FormatQuantityMode::Dot) => mode,
        }
    } else {
        FormatQuantityMode::None
    };

    if peek(cursor).is_some_and(|character| matches!(character.value.0, 'h' | 'l' | 'L')) {
        let _length = advance(cursor);
    }

    let Some(conversion) = advance(cursor) else {
        let index = peek(cursor).map_or(FormatScalarIndex(0), |character| character.index);
        return Err(Failure::Malformed(error(
            FormatErrorKind::IncompleteFormat,
            index,
            None,
        )));
    };
    if !matches!(
        conversion.value.0,
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
pub(super) fn parse_percent_string(input: FormatInput<'_>) -> Result<ParsedFormat, Failure> {
    let mut cursor = self::cursor(input);
    let mut fields = Vec::new();
    let mut summary = FormatSummary::default();
    while let Some(character) = advance(&mut cursor) {
        if character.value.0 != '%' {
            continue;
        }
        let Some(next) = peek(&mut cursor) else {
            return Err(Failure::Malformed(error(
                FormatErrorKind::IncompleteFormat,
                FormatScalarIndex(character.index.0 + 1),
                None,
            )));
        };
        if next.value.0 == '%' {
            let _escaped = advance(&mut cursor);
            continue;
        }
        let parsed = parse_field(&mut cursor, FormatFieldOrdinal(fields.len()))?;
        if matches!(parsed.field.argument, FormatArgument::Mapping(_)) {
            summary.mapping_present = FormatProperty::Present;
        } else {
            summary.positional_consumption += 1;
        }
        if parsed.field.width == FormatQuantityMode::Star {
            summary.positional_consumption += 1;
            summary.dynamic_width = FormatProperty::Present;
        }
        if parsed.field.precision == FormatQuantityMode::Star {
            summary.positional_consumption += 1;
            summary.dynamic_precision = FormatProperty::Present;
        }
        if fields.try_reserve(1).is_err() {
            return Err(Failure::Unavailable);
        }
        fields.push(parsed.field);
    }
    Ok(ParsedFormat { fields, summary })
}
