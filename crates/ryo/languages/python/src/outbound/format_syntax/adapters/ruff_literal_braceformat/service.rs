//! The brace-format grammar: reading replacement fields from a decoded string.

use super::types::{FieldType, Part};
use crate::domain::brace_format::types::{
    BraceArgumentIndex, BraceFieldContext, BraceFieldCount, BraceFormatError, BraceFormatField,
    BraceSelector, BraceSelectorText, BraceSpecText, ParsedBraceFormat,
};
use crate::domain::format_syntax::types::{FormatCharacter, FormatFieldOrdinal, FormatInput};

/// Validate selector syntax and classify its root.
fn parse_field_name(name: FormatInput<'_>) -> Result<FieldType, BraceFormatError> {
    let chars: Vec<char> = name.0.chars().collect();
    let root_end = chars
        .iter()
        .position(|character| matches!(character, '.' | '['))
        .unwrap_or(chars.len());
    let root: String = chars
        .get(..root_end)
        .ok_or(BraceFormatError::UnmatchedBracket)?
        .iter()
        .collect();
    let field_type = if root.is_empty() {
        FieldType::Automatic
    } else if let Ok(index) = root.parse::<usize>() {
        FieldType::Index(BraceArgumentIndex(index), BraceSelectorText(root))
    } else {
        FieldType::Keyword(BraceSelectorText(root))
    };

    let mut at = root_end;
    while let Some(character) = chars.get(at) {
        match character {
            '.' => {
                at += 1;
                let start = at;
                while chars.get(at).is_some_and(|next| !matches!(next, '.' | '[')) {
                    at += 1;
                }
                if start == at {
                    return Err(BraceFormatError::EmptyAttribute);
                }
            }
            '[' => {
                at += 1;
                let start = at;
                while chars.get(at).is_some_and(|next| *next != ']') {
                    at += 1;
                }
                if at == chars.len() {
                    return Err(BraceFormatError::MissingRightBracket);
                }
                if start == at {
                    return Err(BraceFormatError::EmptyAttribute);
                }
                at += 1;
                if chars.get(at).is_some_and(|next| !matches!(next, '.' | '[')) {
                    return Err(BraceFormatError::InvalidCharacterAfterRightBracket);
                }
            }
            _ => return Err(BraceFormatError::InvalidCharacterAfterRightBracket),
        }
    }
    Ok(field_type)
}

/// Split a replacement field into selector, conversion and specification.
fn parse_part(body: FormatInput<'_>) -> Result<Part, BraceFormatError> {
    let mut split = body.0.splitn(2, ':');
    let field = split.next().unwrap_or_default();
    let spec = split.next().map(|spec| BraceSpecText(spec.to_owned()));
    let mut parts = field.splitn(2, '!');
    let name = parts.next().unwrap_or_default();
    let conversion = if let Some(text) = parts.next() {
        let mut chars = text.chars();
        let value = chars.next().ok_or(BraceFormatError::UnknownConversion)?;
        if chars.next().is_some() {
            return Err(BraceFormatError::UnknownConversion);
        }
        Some(FormatCharacter(value))
    } else {
        None
    };
    // FieldName parsing validates field syntax separately, matching Ruff's FormatSummary.
    let _field_type = parse_field_name(FormatInput(name))?;
    Ok(Part::Field {
        name: BraceSelectorText(name.into()),
        spec,
        conversion,
    })
}

/// Find fields while handling escaped braces and one level of nested specifications.
fn parse_parts(text: FormatInput<'_>) -> Result<Vec<Part>, BraceFormatError> {
    let chars: Vec<char> = text.0.chars().collect();
    let mut parts = Vec::new();
    let mut at = 0;
    let mut pending_escape = false;
    while let Some(character) = chars.get(at).copied() {
        if pending_escape
            && character == 'N'
            && chars.get(at + 1) == Some(&'{')
            && let Some(relative_end) = chars
                .get(at + 2..)
                .ok_or(BraceFormatError::UnmatchedBracket)?
                .iter()
                .position(|next| *next == '}')
        {
            at += relative_end + 3;
            pending_escape = false;
            continue;
        }
        if character == '{' && chars.get(at + 1) == Some(&'{') {
            at += 2;
            pending_escape = false;
            continue;
        }
        if character == '}' && chars.get(at + 1) == Some(&'}') {
            at += 2;
            pending_escape = false;
            continue;
        }
        if character == '}' {
            return Err(BraceFormatError::MissingStartBracket);
        }
        if character != '{' {
            pending_escape = character == '\\' && !pending_escape;
            at += 1;
            continue;
        }
        at += 1;
        let start = at;
        let mut nested = false;
        while let Some(member) = chars.get(at) {
            match member {
                '{' if nested => return Err(BraceFormatError::PlaceholderRecursionExceeded),
                '{' => {
                    nested = true;
                    at += 1;
                }
                '}' if nested => {
                    nested = false;
                    at += 1;
                }
                '}' => break,
                _ => at += 1,
            }
        }
        if at == chars.len() {
            return Err(BraceFormatError::UnmatchedBracket);
        }
        let body: String = chars
            .get(start..at)
            .ok_or(BraceFormatError::UnmatchedBracket)?
            .iter()
            .collect();
        at += 1;
        parts.push(parse_part(FormatInput(&body))?);
        pending_escape = false;
    }
    Ok(parts)
}

/// Append a field and update its selector-category count.
fn add_field(
    parsed: &mut ParsedBraceFormat,
    field_name: &BraceSelectorText,
    context: BraceFieldContext,
    format_spec: Option<BraceSpecText>,
    conversion: Option<FormatCharacter>,
) -> Result<FormatFieldOrdinal, BraceFormatError> {
    let field_type = parse_field_name(FormatInput(&field_name.0))?;
    let ordinal = FormatFieldOrdinal(parsed.fields.len());
    let selector = match field_type {
        FieldType::Automatic => {
            let index = parsed.automatic_count.0;
            parsed.automatic_count = parsed
                .automatic_count
                .0
                .checked_add(1)
                .map(BraceFieldCount)
                .ok_or(BraceFormatError::UnmatchedBracket)?;
            BraceSelector::Automatic {
                index: BraceArgumentIndex(index),
                overflow: i64::try_from(index).is_err().into(),
            }
        }
        FieldType::Index(index, text) => {
            parsed.indexed_count = parsed
                .indexed_count
                .0
                .checked_add(1)
                .map(BraceFieldCount)
                .ok_or(BraceFormatError::UnmatchedBracket)?;
            BraceSelector::Index {
                index,
                overflow: i64::try_from(index.0).is_err().into(),
                text,
            }
        }
        FieldType::Keyword(name) => {
            parsed.keyword_count = parsed
                .keyword_count
                .0
                .checked_add(1)
                .map(BraceFieldCount)
                .ok_or(BraceFormatError::UnmatchedBracket)?;
            BraceSelector::Keyword(name)
        }
    };
    parsed.fields.push(BraceFormatField {
        ordinal,
        context,
        selector,
        format_spec,
        selector_text: field_name.clone(),
        conversion,
    });
    Ok(ordinal)
}

/// Parse a complete decoded format string with nested field provenance.
pub(super) fn parse(input: FormatInput<'_>) -> Result<ParsedBraceFormat, BraceFormatError> {
    let parts = parse_parts(input)?;
    let mut parsed = ParsedBraceFormat::default();
    for part in parts {
        let Part::Field {
            name,
            spec,
            conversion,
        } = part;
        let parent = add_field(
            &mut parsed,
            &name,
            BraceFieldContext::Literal,
            spec.clone(),
            conversion,
        )?;
        let spec_text = spec.as_ref().map_or(String::new(), |spec| spec.0.clone());
        for nested in parse_parts(FormatInput(&spec_text))? {
            let Part::Field {
                name: nested_name,
                spec: nested_spec,
                conversion: nested_conversion,
            } = nested;
            let _ordinal = add_field(
                &mut parsed,
                &nested_name,
                BraceFieldContext::FormatSpec {
                    parent_ordinal: parent,
                },
                nested_spec,
                nested_conversion,
            )?;
            parsed.nested_field_count = parsed
                .nested_field_count
                .0
                .checked_add(1)
                .map(BraceFieldCount)
                .ok_or(BraceFormatError::UnmatchedBracket)?;
        }
    }
    Ok(parsed)
}
