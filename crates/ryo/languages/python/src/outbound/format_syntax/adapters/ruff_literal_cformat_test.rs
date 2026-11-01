//! Regression cases for ruff literal cformat bindings.
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
