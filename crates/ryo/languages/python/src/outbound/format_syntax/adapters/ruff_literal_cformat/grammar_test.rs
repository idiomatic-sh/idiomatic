//! Regression cases for ruff literal cformat bindings.
use super::grammar::RuffLiteralCFormat;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::format_syntax::interface::FormatSyntax;
use crate::domain::format_syntax::service::flag_text;
use crate::domain::format_syntax::types::{
    FormatArgument, FormatDialect, FormatErrorKind, FormatParseError, FormatParseResult,
    FormatQuantityMode, FormatSummary, ParsedFormat,
};

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

fn escaped_percent_is_a_valid_zero_field_format() {
    let result = parsed("100%%");
    assert!(result.fields.is_empty());
    assert_eq!(result.summary, FormatSummary::default());
}

fn dynamic_width_and_precision_each_consume_positional_values() {
    let result = parsed("%*.*s");
    assert_eq!(result.summary.positional_consumption.0, 3);
    assert!(bool::from(result.summary.dynamic_width));
    assert!(bool::from(result.summary.dynamic_precision));
    assert_eq!(result.fields[0].width, FormatQuantityMode::Star);
    assert_eq!(result.fields[0].precision, FormatQuantityMode::Star);
}

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

/// Which of the file's cases a row runs.
#[derive(Clone, Copy, Debug)]
enum Body {
    /// The case `escaped_percent_is_a_valid_zero_field_format`.
    EscapedPercentIsAValidZeroFieldFormat,
    /// The case `dynamic_width_and_precision_each_consume_positional_values`.
    DynamicWidthAndPrecisionEachConsumePositionalValues,
    /// The case `repeated_mapping_keys_and_mixed_positional_fields_are_preserved`.
    RepeatedMappingKeysAndMixedPositionalFieldsArePreserved,
    /// The case `malformed_percent_and_unicode_error_indices_use_scalar_offsets`.
    MalformedPercentAndUnicodeErrorIndicesUseScalarOffsets,
    /// The case `unmatched_mapping_and_overflow_are_diagnostics`.
    UnmatchedMappingAndOverflowAreDiagnostics,
    /// The case `flags_are_preserved_without_reordering_the_grammar_or_applying_precedence`.
    FlagsArePreservedWithoutReorderingTheGrammarOrApplyingPrecedence,
}

/// The rows of the cases whose bodies assert for themselves.
const CASES: &[Case<Body, (), ()>] = &[
    Case {
        name: CaseName("escaped_percent_is_a_valid_zero_field_format"),
        word: Word::Succeeds,
        input: || Body::EscapedPercentIsAValidZeroFieldFormat,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("dynamic_width_and_precision_each_consume_positional_values"),
        word: Word::Succeeds,
        input: || Body::DynamicWidthAndPrecisionEachConsumePositionalValues,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("repeated_mapping_keys_and_mixed_positional_fields_are_preserved"),
        word: Word::Succeeds,
        input: || Body::RepeatedMappingKeysAndMixedPositionalFieldsArePreserved,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("malformed_percent_and_unicode_error_indices_use_scalar_offsets"),
        word: Word::Succeeds,
        input: || Body::MalformedPercentAndUnicodeErrorIndicesUseScalarOffsets,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unmatched_mapping_and_overflow_are_diagnostics"),
        word: Word::Succeeds,
        input: || Body::UnmatchedMappingAndOverflowAreDiagnostics,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("flags_are_preserved_without_reordering_the_grammar_or_applying_precedence"),
        word: Word::Succeeds,
        input: || Body::FlagsArePreservedWithoutReorderingTheGrammarOrApplyingPrecedence,
        expect: Expect::Value(()),
    },
];

/// Runs the body of one case.
fn body(given: Body) -> Result<(), ()> {
    match given {
        Body::EscapedPercentIsAValidZeroFieldFormat => {
            escaped_percent_is_a_valid_zero_field_format()
        }
        Body::DynamicWidthAndPrecisionEachConsumePositionalValues => {
            dynamic_width_and_precision_each_consume_positional_values()
        }
        Body::RepeatedMappingKeysAndMixedPositionalFieldsArePreserved => {
            repeated_mapping_keys_and_mixed_positional_fields_are_preserved()
        }
        Body::MalformedPercentAndUnicodeErrorIndicesUseScalarOffsets => {
            malformed_percent_and_unicode_error_indices_use_scalar_offsets()
        }
        Body::UnmatchedMappingAndOverflowAreDiagnostics => {
            unmatched_mapping_and_overflow_are_diagnostics()
        }
        Body::FlagsArePreservedWithoutReorderingTheGrammarOrApplyingPrecedence => {
            flags_are_preserved_without_reordering_the_grammar_or_applying_precedence()
        }
    }
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, body);
}
