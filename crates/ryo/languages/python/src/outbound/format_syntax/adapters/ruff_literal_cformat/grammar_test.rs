//! Regression cases for ruff literal cformat bindings.
use super::grammar::RuffLiteralCFormat;
use crate::domain::format_syntax::types::{
    FormatArgument, FormatDialect, FormatErrorKind, FormatParseError, FormatParseResult,
    FormatQuantityMode, FormatSummary, ParsedFormat,
};
use crate::domain::format_syntax::interface::FormatSyntax;
use crate::domain::format_syntax::service::flag_text;

fn parsed(source: &str) -> ParsedFormat {
    match RuffLiteralCFormat::parse(
        crate::domain::format_syntax::types::FormatInput(source),
        FormatDialect::PercentString,
    ) {
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
    assert_eq!(result.summary.positional_consumption.0, 3);
    assert!(bool::from(result.summary.dynamic_width));
    assert!(bool::from(result.summary.dynamic_precision));
    assert_eq!(result.fields[0].width, FormatQuantityMode::Star);
    assert_eq!(result.fields[0].precision, FormatQuantityMode::Star);
}

#[test]
fn repeated_mapping_keys_and_mixed_positional_fields_are_preserved() {
    let result = parsed("%(name)s %(name)r %s");
    assert_eq!(result.fields.len(), 3);
    assert_eq!(
        result.fields[0].argument,
        FormatArgument::Mapping(crate::domain::format_syntax::types::FormatMappingKey(
            "name".to_owned()
        ))
    );
    assert_eq!(
        result.fields[1].argument,
        FormatArgument::Mapping(crate::domain::format_syntax::types::FormatMappingKey(
            "name".to_owned()
        ))
    );
    assert_eq!(result.fields[2].argument, FormatArgument::Positional);
    assert_eq!(result.summary.positional_consumption.0, 1);
    assert!(bool::from(result.summary.mapping_present));
}

#[test]
fn malformed_percent_and_unicode_error_indices_use_scalar_offsets() {
    assert_eq!(
        RuffLiteralCFormat::parse(
            crate::domain::format_syntax::types::FormatInput("é%"),
            FormatDialect::PercentString
        ),
        FormatParseResult::Malformed(FormatParseError {
            kind: FormatErrorKind::IncompleteFormat,
            character: None,
            index: crate::domain::format_syntax::types::FormatScalarIndex(2),
        }),
    );
    assert_eq!(
        RuffLiteralCFormat::parse(
            crate::domain::format_syntax::types::FormatInput("é%n"),
            FormatDialect::PercentString
        ),
        FormatParseResult::Malformed(FormatParseError {
            kind: FormatErrorKind::UnsupportedFormatCharacter,
            character: Some(crate::domain::format_syntax::types::FormatCharacter('n')),
            index: crate::domain::format_syntax::types::FormatScalarIndex(2),
        }),
    );
}

#[test]
fn unmatched_mapping_and_overflow_are_diagnostics() {
    assert!(matches!(
        RuffLiteralCFormat::parse(
            crate::domain::format_syntax::types::FormatInput("%(name"),
            FormatDialect::PercentString
        ),
        FormatParseResult::Malformed(FormatParseError {
            kind: FormatErrorKind::UnmatchedKeyParentheses,
            ..
        })
    ));
    assert!(matches!(
        RuffLiteralCFormat::parse(
            crate::domain::format_syntax::types::FormatInput("%999999999999999999999s"),
            FormatDialect::PercentString
        ),
        FormatParseResult::Malformed(FormatParseError {
            kind: FormatErrorKind::WidthOrPrecisionTooLarge,
            ..
        })
    ));
    assert_eq!(
        RuffLiteralCFormat::parse(
            crate::domain::format_syntax::types::FormatInput("%*"),
            FormatDialect::PercentString
        ),
        FormatParseResult::Malformed(FormatParseError {
            kind: FormatErrorKind::IncompleteFormat,
            character: None,
            index: crate::domain::format_syntax::types::FormatScalarIndex(0),
        }),
    );
    assert!(matches!(
        RuffLiteralCFormat::parse(
            crate::domain::format_syntax::types::FormatInput("%b"),
            FormatDialect::PercentString
        ),
        FormatParseResult::Malformed(FormatParseError {
            kind: FormatErrorKind::UnsupportedFormatCharacter,
            character: Some(crate::domain::format_syntax::types::FormatCharacter('b')),
            ..
        })
    ));
}

#[test]
fn flags_are_preserved_without_reordering_the_grammar_or_applying_precedence() {
    let plain = parsed("%s");
    assert_eq!(flag_text(plain.fields[0].flags).0, "");
    let combined = parsed("%0- +#10r");
    assert_eq!(flag_text(combined.fields[0].flags).0, " #+-0");
    assert_eq!(combined.fields[0].width, FormatQuantityMode::Fixed);
    assert_eq!(combined.fields[0].precision, FormatQuantityMode::None);
    let repeated = parsed("%##00--++  r");
    assert_eq!(repeated.fields[0].flags, combined.fields[0].flags);
    assert_eq!(repeated.fields[0].width, FormatQuantityMode::None);
    let zero = parsed("%0s");
    assert_eq!(flag_text(zero.fields[0].flags).0, "0");
    assert_eq!(zero.fields[0].width, FormatQuantityMode::None);
    assert!(parsed("%%").fields.is_empty());
}
