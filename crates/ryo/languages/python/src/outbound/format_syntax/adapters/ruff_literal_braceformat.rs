//! A minimal adapter of Ruff 0.16.5's `ruff_python_literal::format` grammar.
//!
//! The field and brace grammar is adapted from `crates/ruff_python_literal/src/format.rs`
//! (MIT license, Astral Software Inc. and contributors). This adapter keeps only field roots,
//! nesting, summary counts and generic parser categories needed by Python brace-format readers.
use crate::domain::brace_format::interface::BraceFormatSyntax;
use crate::domain::brace_format::types::{
    BraceFieldContext, BraceFormatError, BraceFormatField, BraceFormatResult, BraceSelector,
    ParsedBraceFormat,
};

/// The parser selected for decoded Python brace-format strings.
pub(crate) struct RuffLiteralBraceFormat;

#[derive(Clone, Debug)]
/// A replacement field retained by the brace grammar.
enum Part {
    /// A field root and its optional nested formatting specification.
    Field {
        /// The field's selector and conversion.
        name: String,
        /// The text following the field's colon.
        spec: Option<String>,
        conversion: Option<char>,
    },
}

#[derive(Clone, Debug)]
/// The root selector before attribute and subscript operations.
enum FieldType {
    /// An omitted selector using implicit positional numbering.
    Automatic,
    /// A parsed positional index with its original spelling.
    Index(usize, String),
    /// A keyword selector.
    Keyword(String),
}

/// Validate selector syntax and classify its root.
fn parse_field_name(name: &str) -> Result<FieldType, BraceFormatError> {
    let chars: Vec<char> = name.chars().collect();
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
        FieldType::Index(index, root)
    } else {
        FieldType::Keyword(root)
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
fn parse_part(body: &str) -> Result<Part, BraceFormatError> {
    let mut split = body.splitn(2, ':');
    let field = split.next().unwrap_or_default();
    let spec = split.next().map(str::to_owned);
    let mut parts = field.splitn(2, '!');
    let name = parts.next().unwrap_or_default();
    let conversion = if let Some(text) = parts.next() {
        let mut chars = text.chars();
        let value = chars.next().ok_or(BraceFormatError::UnknownConversion)?;
        if chars.next().is_some() {
            return Err(BraceFormatError::UnknownConversion);
        }
        Some(value)
    } else {
        None
    };
    // FieldName parsing validates field syntax separately, matching Ruff's FormatSummary.
    let _field_type = parse_field_name(name)?;
    Ok(Part::Field {
        name: name.into(),
        spec,
        conversion,
    })
}

/// Find fields while handling escaped braces and one level of nested specifications.
fn parse_parts(text: &str) -> Result<Vec<Part>, BraceFormatError> {
    let chars: Vec<char> = text.chars().collect();
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
        parts.push(parse_part(&body)?);
        pending_escape = false;
    }
    Ok(parts)
}

/// Append a field and update its selector-category count.
fn add_field(
    parsed: &mut ParsedBraceFormat,
    field_name: &str,
    context: BraceFieldContext,
    format_spec: Option<String>,
    conversion: Option<char>,
) -> Result<usize, BraceFormatError> {
    let field_type = parse_field_name(field_name)?;
    let ordinal = parsed.fields.len();
    let selector = match field_type {
        FieldType::Automatic => {
            let index = parsed.automatic_count;
            parsed.automatic_count = parsed
                .automatic_count
                .checked_add(1)
                .ok_or(BraceFormatError::UnmatchedBracket)?;
            BraceSelector::Automatic {
                index,
                overflow: i64::try_from(index).is_err(),
            }
        }
        FieldType::Index(index, text) => {
            parsed.indexed_count = parsed
                .indexed_count
                .checked_add(1)
                .ok_or(BraceFormatError::UnmatchedBracket)?;
            BraceSelector::Index {
                index,
                overflow: i64::try_from(index).is_err(),
                text,
            }
        }
        FieldType::Keyword(name) => {
            parsed.keyword_count = parsed
                .keyword_count
                .checked_add(1)
                .ok_or(BraceFormatError::UnmatchedBracket)?;
            BraceSelector::Keyword(name)
        }
    };
    parsed.fields.push(BraceFormatField {
        ordinal,
        context,
        selector,
        format_spec,
        selector_text: field_name.to_owned(),
        conversion,
    });
    Ok(ordinal)
}

/// Parse a complete decoded format string with nested field provenance.
fn parse(input: &str) -> Result<ParsedBraceFormat, BraceFormatError> {
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
        for nested in parse_parts(spec.as_deref().unwrap_or_default())? {
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
                .checked_add(1)
                .ok_or(BraceFormatError::UnmatchedBracket)?;
        }
    }
    Ok(parsed)
}

impl BraceFormatSyntax for RuffLiteralBraceFormat {
    fn parse(input: &str) -> BraceFormatResult {
        match parse(input) {
            Ok(parsed) => BraceFormatResult::Parsed(parsed),
            Err(error) => BraceFormatResult::Malformed(error),
        }
    }
}

#[cfg(test)]
#[path = "ruff_literal_braceformat_test.rs"]
mod tests;
