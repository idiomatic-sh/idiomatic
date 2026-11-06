//! The cformat grammar: reading conversion specifications from a decoded string.

use super::types::{Character, Cursor, Failure, ParsedField};
use crate::domain::format_syntax::service::insert_flag;
use crate::domain::format_syntax::types::{
    FlagRecognition, FormatArgument, FormatCharacter, FormatErrorKind, FormatFieldOrdinal,
    FormatFieldValue, FormatFlags, FormatMappingKey, FormatParseError, FormatProperty,
    FormatQuantityMode, FormatScalarIndex, FormatSummary, ParsedFormat,
};

/// Begin reading one decoded input.
pub(super) fn cursor(input: &str) -> Cursor<'_> {
    Cursor {
        characters: input.chars().enumerate(),
        peeked: None,
    }
}

/// Borrow the next scalar without consuming it.
pub(super) fn peek(cursor: &mut Cursor<'_>) -> Option<Character> {
    if cursor.peeked.is_none() {
        cursor.peeked = cursor
            .characters
            .next()
            .map(|(index, value)| Character { index, value });
    }
    cursor.peeked
}

/// Consume the next scalar, including any retained lookahead.
pub(super) fn advance(cursor: &mut Cursor<'_>) -> Option<Character> {
    cursor.peeked.take().or_else(|| {
        cursor
            .characters
            .next()
            .map(|(index, value)| Character { index, value })
    })
}

/// Construct a grammar diagnostic with a scalar-based index.
fn error(kind: FormatErrorKind, index: usize, character: Option<char>) -> FormatParseError {
    FormatParseError {
        kind,
        character: character.map(FormatCharacter),
        index: FormatScalarIndex(index),
    }
}

/// Consume an optional mapping key, respecting nested parentheses.
fn mapping_key(cursor: &mut Cursor<'_>) -> Result<Option<String>, Failure> {
    let Some(open) = peek(cursor) else {
        return Ok(None);
    };
    if open.value != '(' {
        return Ok(None);
    }
    let _open = advance(cursor);
    let mut depth = 1_usize;
    let mut key = String::new();
    while let Some(character) = advance(cursor) {
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
    match peek(cursor) {
        Some(character) if character.value == '*' => {
            let _star = advance(cursor);
            Ok(FormatQuantityMode::Star)
        }
        Some(first) if first.value.is_ascii_digit() => {
            let mut amount = 0_i32;
            while let Some(character) = peek(cursor).filter(|each| each.value.is_ascii_digit()) {
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
                let _digit = advance(cursor);
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
    while peek(cursor).is_some_and(|character| {
        insert_flag(&mut flags, FormatCharacter(character.value)) == FlagRecognition::Consumed
    }) {
        let _flag = advance(cursor);
    }

    let width = quantity(cursor)?;
    let precision = if peek(cursor)
        .is_some_and(|character| character.value == '.')
    {
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

    if peek(cursor)
        .is_some_and(|character| matches!(character.value, 'h' | 'l' | 'L'))
    {
        let _length = advance(cursor);
    }

    let Some(conversion) = advance(cursor) else {
        let index = peek(cursor).map_or(0, |character| character.index);
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
            ordinal: FormatFieldOrdinal(ordinal),
            argument: key.map_or(FormatArgument::Positional, |key| {
                FormatArgument::Mapping(FormatMappingKey(key))
            }),
            conversion: FormatCharacter(conversion.value),
            width,
            precision,
            flags,
        },
    })
}

/// Parse all conversions and retain their ordered argument consumption summary.
pub(super) fn parse_percent_string(input: &str) -> Result<ParsedFormat, Failure> {
    let mut cursor = self::cursor(input);
    let mut fields = Vec::new();
    let mut summary = FormatSummary::default();
    while let Some(character) = advance(&mut cursor) {
        if character.value != '%' {
            continue;
        }
        let Some(next) = peek(&mut cursor) else {
            return Err(Failure::Malformed(error(
                FormatErrorKind::IncompleteFormat,
                character.index + 1,
                None,
            )));
        };
        if next.value == '%' {
            let _escaped = advance(&mut cursor);
            continue;
        }
        let parsed = parse_field(&mut cursor, fields.len())?;
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
