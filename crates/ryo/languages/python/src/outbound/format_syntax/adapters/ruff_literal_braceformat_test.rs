//! Regression cases for ruff literal braceformat bindings.
use super::{RuffLiteralBraceFormat, parse};
use crate::domain::brace_format::interface::BraceFormatSyntax as _;
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
    let huge = format!(
        "{{{}}}",
        "9".repeat(usize::try_from(usize::BITS).unwrap_or(usize::MAX) / 3 + 2)
    );
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
