//! Regression cases for ruff literal braceformat bindings.
use super::grammar::RuffLiteralBraceFormat;
use super::service::parse;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::brace_format::types::{
    BraceFieldContext, BraceFormatError, BraceFormatResult, BraceSelector,
};
use crate::domain::format_syntax::types::FormatInput;
use crate::outbound::brace_format::interface::BraceFormatSyntaxPort as _;

fn preserves_selector_kinds_duplicates_and_nested_order() {
    let parsed = parse(FormatInput("{{{key!r:{width}} {} {2} {2}")).expect("valid format");
    assert_eq!(parsed.automatic_count.0, 1);
    assert_eq!(parsed.indexed_count.0, 2);
    assert_eq!(parsed.keyword_count.0, 2);
    assert_eq!(parsed.nested_field_count.0, 1);
    assert_eq!(parsed.fields[0].context, BraceFieldContext::Literal);
    assert_eq!(
        parsed.fields[0].selector,
        BraceSelector::Keyword(crate::domain::brace_format::types::BraceSelectorText(
            "key".into()
        ))
    );
    assert_eq!(
        parsed.fields[1].context,
        BraceFieldContext::FormatSpec {
            parent_ordinal: crate::domain::format_syntax::types::FormatFieldOrdinal(0)
        }
    );
    assert_eq!(
        parsed.fields[1].selector,
        BraceSelector::Keyword(crate::domain::brace_format::types::BraceSelectorText(
            "width".into()
        ))
    );
    assert_eq!(
        parsed.fields[2].selector,
        BraceSelector::Automatic {
            index: crate::domain::brace_format::types::BraceArgumentIndex(0),
            overflow: crate::domain::brace_format::types::SelectorRange::Representable
        }
    );
    assert_eq!(
        parsed.fields[3].selector,
        BraceSelector::Index {
            index: crate::domain::brace_format::types::BraceArgumentIndex(2),
            overflow: crate::domain::brace_format::types::SelectorRange::Representable,
            text: crate::domain::brace_format::types::BraceSelectorText("2".into())
        }
    );
    assert_eq!(
        parsed.fields[4].selector,
        BraceSelector::Index {
            index: crate::domain::brace_format::types::BraceArgumentIndex(2),
            overflow: crate::domain::brace_format::types::SelectorRange::Representable,
            text: crate::domain::brace_format::types::BraceSelectorText("2".into())
        }
    );
}

fn retains_decoded_spec_text_and_explicit_empty_colon() {
    let parsed = parse(FormatInput("{thing:.{width}q} {plain:} {absent}")).expect("valid format");
    assert_eq!(
        parsed.fields[0]
            .format_spec
            .as_ref()
            .map(|text| text.0.as_str()),
        Some(".{width}q")
    );
    assert_eq!(
        parsed.fields[1].context,
        BraceFieldContext::FormatSpec {
            parent_ordinal: crate::domain::format_syntax::types::FormatFieldOrdinal(0)
        }
    );
    assert_eq!(parsed.fields[1].format_spec, None);
    assert_eq!(
        parsed.fields[2]
            .format_spec
            .as_ref()
            .map(|text| text.0.as_str()),
        Some("")
    );
    assert_eq!(parsed.fields[3].format_spec, None);
}

fn distinguishes_valid_empty_from_malformed_and_digit_keyword() {
    assert!(
        matches!(RuffLiteralBraceFormat::parse(crate::domain::format_syntax::types::FormatInput("{{}}")), BraceFormatResult::Parsed(parsed) if parsed.fields.is_empty())
    );
    assert!(matches!(
        RuffLiteralBraceFormat::parse(crate::domain::format_syntax::types::FormatInput(
            "{key..name}"
        )),
        BraceFormatResult::Malformed(BraceFormatError::EmptyAttribute)
    ));
    let huge = format!(
        "{{{}}}",
        "9".repeat(usize::try_from(usize::BITS).unwrap_or(usize::MAX) / 3 + 2)
    );
    assert!(
        matches!(RuffLiteralBraceFormat::parse(crate::domain::format_syntax::types::FormatInput(&huge)), BraceFormatResult::Parsed(parsed) if matches!(&parsed.fields[0].selector, BraceSelector::Keyword(_)))
    );
    let above_number = "{9223372036854775808}";
    if usize::BITS >= i64::BITS {
        assert!(matches!(
            RuffLiteralBraceFormat::parse(crate::domain::format_syntax::types::FormatInput(above_number)),
            BraceFormatResult::Parsed(parsed)
                if matches!(parsed.fields.first().map(|field| &field.selector), Some(BraceSelector::Index { overflow: crate::domain::brace_format::types::SelectorRange::Overflow, text, .. }) if text.0 == "9223372036854775808")
        ));
    } else {
        assert!(matches!(
            RuffLiteralBraceFormat::parse(crate::domain::format_syntax::types::FormatInput(above_number)),
            BraceFormatResult::Parsed(parsed)
                if matches!(parsed.fields.first().map(|field| &field.selector), Some(BraceSelector::Keyword(_)))
        ));
    }
    assert!(
        matches!(RuffLiteralBraceFormat::parse(crate::domain::format_syntax::types::FormatInput(r"\N{snowman}")), BraceFormatResult::Parsed(parsed) if parsed.fields.is_empty())
    );
}

fn reports_nesting_and_unmatched_braces() {
    assert!(matches!(
        parse(FormatInput("{x:{y:{z}}}")),
        Err(BraceFormatError::PlaceholderRecursionExceeded)
    ));
    assert!(matches!(
        parse(FormatInput("{x")),
        Err(BraceFormatError::UnmatchedBracket)
    ));
    assert!(matches!(
        parse(FormatInput("x}")),
        Err(BraceFormatError::MissingStartBracket)
    ));
}

fn full_selector_paths_and_conversions_survive_spec_retention() {
    let BraceFormatResult::Parsed(parsed) = RuffLiteralBraceFormat::parse(
        crate::domain::format_syntax::types::FormatInput("{0.value!s:{width!r:q}}"),
    ) else {
        panic!("valid brace grammar");
    };
    assert_eq!(parsed.fields[0].selector_text.0, "0.value");
    assert_eq!(
        parsed.fields[0].conversion,
        Some(crate::domain::format_syntax::types::FormatCharacter('s'))
    );
    assert_eq!(parsed.fields[1].selector_text.0, "width");
    assert_eq!(
        parsed.fields[1].conversion,
        Some(crate::domain::format_syntax::types::FormatCharacter('r'))
    );
    assert_eq!(
        parsed.fields[1]
            .format_spec
            .as_ref()
            .map(|text| text.0.as_str()),
        Some("q")
    );
}

/// Which of the file's cases a row runs.
#[derive(Clone, Copy, Debug)]
enum Body {
    /// The case `preserves_selector_kinds_duplicates_and_nested_order`.
    PreservesSelectorKindsDuplicatesAndNestedOrder,
    /// The case `retains_decoded_spec_text_and_explicit_empty_colon`.
    RetainsDecodedSpecTextAndExplicitEmptyColon,
    /// The case `distinguishes_valid_empty_from_malformed_and_digit_keyword`.
    DistinguishesValidEmptyFromMalformedAndDigitKeyword,
    /// The case `reports_nesting_and_unmatched_braces`.
    ReportsNestingAndUnmatchedBraces,
    /// The case `full_selector_paths_and_conversions_survive_spec_retention`.
    FullSelectorPathsAndConversionsSurviveSpecRetention,
}

/// The rows of the cases whose bodies assert for themselves.
const CASES: &[Case<Body, (), ()>] = &[
    Case {
        name: CaseName("preserves_selector_kinds_duplicates_and_nested_order"),
        word: Word::Succeeds,
        input: || Body::PreservesSelectorKindsDuplicatesAndNestedOrder,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("retains_decoded_spec_text_and_explicit_empty_colon"),
        word: Word::Succeeds,
        input: || Body::RetainsDecodedSpecTextAndExplicitEmptyColon,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("distinguishes_valid_empty_from_malformed_and_digit_keyword"),
        word: Word::Succeeds,
        input: || Body::DistinguishesValidEmptyFromMalformedAndDigitKeyword,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("reports_nesting_and_unmatched_braces"),
        word: Word::Succeeds,
        input: || Body::ReportsNestingAndUnmatchedBraces,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("full_selector_paths_and_conversions_survive_spec_retention"),
        word: Word::Succeeds,
        input: || Body::FullSelectorPathsAndConversionsSurviveSpecRetention,
        expect: Expect::Value(()),
    },
];

/// Runs the body of one case.
fn body(given: Body) -> Result<(), ()> {
    match given {
        Body::PreservesSelectorKindsDuplicatesAndNestedOrder => {
            preserves_selector_kinds_duplicates_and_nested_order()
        }
        Body::RetainsDecodedSpecTextAndExplicitEmptyColon => {
            retains_decoded_spec_text_and_explicit_empty_colon()
        }
        Body::DistinguishesValidEmptyFromMalformedAndDigitKeyword => {
            distinguishes_valid_empty_from_malformed_and_digit_keyword()
        }
        Body::ReportsNestingAndUnmatchedBraces => reports_nesting_and_unmatched_braces(),
        Body::FullSelectorPathsAndConversionsSurviveSpecRetention => {
            full_selector_paths_and_conversions_survive_spec_retention()
        }
    }
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, body);
}
