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
enum Part {
    Field { name: String, spec: String },
}

#[derive(Clone, Debug)]
enum FieldType {
    Automatic,
    Index(usize, String),
    Keyword(String),
}

fn parse_field_name(name: &str) -> Result<FieldType, BraceFormatError> {
    let chars: Vec<char> = name.chars().collect();
    let root_end = chars
        .iter()
        .position(|character| matches!(character, '.' | '['))
        .unwrap_or(chars.len());
    let root: String = chars[..root_end].iter().collect();
    let field_type = if root.is_empty() {
        FieldType::Automatic
    } else if let Ok(index) = root.parse::<usize>() {
        FieldType::Index(index, root)
    } else {
        FieldType::Keyword(root)
    };

    let mut at = root_end;
    while at < chars.len() {
        match chars[at] {
            '.' => {
                at += 1;
                let start = at;
                while at < chars.len() && !matches!(chars[at], '.' | '[') {
                    at += 1;
                }
                if start == at {
                    return Err(BraceFormatError::EmptyAttribute);
                }
            }
            '[' => {
                at += 1;
                let start = at;
                while at < chars.len() && chars[at] != ']' {
                    at += 1;
                }
                if at == chars.len() {
                    return Err(BraceFormatError::MissingRightBracket);
                }
                if start == at {
                    return Err(BraceFormatError::EmptyAttribute);
                }
                at += 1;
                if at < chars.len() && !matches!(chars[at], '.' | '[') {
                    return Err(BraceFormatError::InvalidCharacterAfterRightBracket);
                }
            }
            _ => return Err(BraceFormatError::InvalidCharacterAfterRightBracket),
        }
    }
    Ok(field_type)
}

fn parse_part(body: &str) -> Result<Part, BraceFormatError> {
    let mut split = body.splitn(2, ':');
    let field = split.next().unwrap_or_default();
    let spec = split.next().unwrap_or_default();
    let mut conversion = field.splitn(2, '!');
    let name = conversion.next().unwrap_or_default();
    if let Some(conversion) = conversion.next() {
        let mut chars = conversion.chars();
        if chars.next().is_none() || chars.next().is_some() {
            return Err(BraceFormatError::UnknownConversion);
        }
    }
    // FieldName parsing validates field syntax separately, matching Ruff's FormatSummary.
    let _field_type = parse_field_name(name)?;
    Ok(Part::Field {
        name: name.into(),
        spec: spec.into(),
    })
}

fn parse_parts(text: &str) -> Result<Vec<Part>, BraceFormatError> {
    let chars: Vec<char> = text.chars().collect();
    let mut parts = Vec::new();
    let mut at = 0;
    let mut pending_escape = false;
    while at < chars.len() {
        if pending_escape
            && chars[at] == 'N'
            && chars.get(at + 1) == Some(&'{')
            && let Some(relative_end) = chars[at + 2..]
                .iter()
                .position(|character| *character == '}')
        {
            at += relative_end + 3;
            pending_escape = false;
            continue;
        }
        if chars[at] == '{' && chars.get(at + 1) == Some(&'{') {
            at += 2;
            pending_escape = false;
            continue;
        }
        if chars[at] == '}' && chars.get(at + 1) == Some(&'}') {
            at += 2;
            pending_escape = false;
            continue;
        }
        if chars[at] == '}' {
            return Err(BraceFormatError::MissingStartBracket);
        }
        if chars[at] != '{' {
            let character = chars[at];
            pending_escape = character == '\\' && !pending_escape;
            at += 1;
            continue;
        }
        at += 1;
        let start = at;
        let mut nested = false;
        while at < chars.len() {
            match chars[at] {
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
        let body: String = chars[start..at].iter().collect();
        at += 1;
        parts.push(parse_part(&body)?);
        pending_escape = false;
    }
    Ok(parts)
}

fn add_field(
    parsed: &mut ParsedBraceFormat,
    field_name: &str,
    context: BraceFieldContext,
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
    });
    Ok(ordinal)
}

fn parse(input: &str) -> Result<ParsedBraceFormat, BraceFormatError> {
    let parts = parse_parts(input)?;
    let mut parsed = ParsedBraceFormat::default();
    for part in parts {
        let Part::Field { name, spec } = part;
        let parent = add_field(&mut parsed, &name, BraceFieldContext::Literal)?;
        for nested in parse_parts(&spec)? {
            let Part::Field { name, .. } = nested;
            let _ordinal = add_field(
                &mut parsed,
                &name,
                BraceFieldContext::FormatSpec {
                    parent_ordinal: parent,
                },
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
mod tests {
    use super::{RuffLiteralBraceFormat, parse};
    use crate::domain::brace_format::interface::BraceFormatSyntax;
    use crate::domain::brace_format::types::{
        BraceFieldContext, BraceFormatError, BraceFormatResult, BraceSelector,
    };

    #[test]
    fn preserves_selector_kinds_duplicates_and_nested_order() {
        let parsed = parse("{{{key!r:{width}} {} {2} {2}").expect("valid format");
        assert_eq!(parsed.automatic_count, 1);
        assert_eq!(parsed.indexed_count, 2);
        assert_eq!(parsed.keyword_count, 2);
        assert_eq!(parsed.nested_field_count, 1);
        assert_eq!(parsed.fields[0].context, BraceFieldContext::Literal);
        assert_eq!(
            parsed.fields[0].selector,
            BraceSelector::Keyword("key".into())
        );
        assert_eq!(
            parsed.fields[1].context,
            BraceFieldContext::FormatSpec { parent_ordinal: 0 }
        );
        assert_eq!(
            parsed.fields[1].selector,
            BraceSelector::Keyword("width".into())
        );
        assert_eq!(
            parsed.fields[2].selector,
            BraceSelector::Automatic {
                index: 0,
                overflow: false
            }
        );
        assert_eq!(
            parsed.fields[3].selector,
            BraceSelector::Index {
                index: 2,
                overflow: false,
                text: "2".into()
            }
        );
        assert_eq!(
            parsed.fields[4].selector,
            BraceSelector::Index {
                index: 2,
                overflow: false,
                text: "2".into()
            }
        );
    }

    #[test]
    fn distinguishes_valid_empty_from_malformed_and_digit_keyword() {
        assert!(
            matches!(RuffLiteralBraceFormat::parse("{{}}"), BraceFormatResult::Parsed(parsed) if parsed.fields.is_empty())
        );
        assert!(matches!(
            RuffLiteralBraceFormat::parse("{key..name}"),
            BraceFormatResult::Malformed(BraceFormatError::EmptyAttribute)
        ));
        let huge = format!("{{{}}}", "9".repeat(usize::BITS as usize / 3 + 2));
        assert!(
            matches!(RuffLiteralBraceFormat::parse(&huge), BraceFormatResult::Parsed(parsed) if matches!(&parsed.fields[0].selector, BraceSelector::Keyword(_)))
        );
        let above_number = "{9223372036854775808}";
        if usize::BITS >= i64::BITS {
            assert!(matches!(
                RuffLiteralBraceFormat::parse(above_number),
                BraceFormatResult::Parsed(parsed)
                    if matches!(parsed.fields.first().map(|field| &field.selector), Some(BraceSelector::Index { overflow: true, text, .. }) if text == "9223372036854775808")
            ));
        } else {
            assert!(matches!(
                RuffLiteralBraceFormat::parse(above_number),
                BraceFormatResult::Parsed(parsed)
                    if matches!(parsed.fields.first().map(|field| &field.selector), Some(BraceSelector::Keyword(_)))
            ));
        }
        assert!(
            matches!(RuffLiteralBraceFormat::parse(r"\N{snowman}"), BraceFormatResult::Parsed(parsed) if parsed.fields.is_empty())
        );
    }

    #[test]
    fn reports_nesting_and_unmatched_braces() {
        assert!(matches!(
            parse("{x:{y:{z}}}"),
            Err(BraceFormatError::PlaceholderRecursionExceeded)
        ));
        assert!(matches!(
            parse("{x"),
            Err(BraceFormatError::UnmatchedBracket)
        ));
        assert!(matches!(
            parse("x}"),
            Err(BraceFormatError::MissingStartBracket)
        ));
    }
}
