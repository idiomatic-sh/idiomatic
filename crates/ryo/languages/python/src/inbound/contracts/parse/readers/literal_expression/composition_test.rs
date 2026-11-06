//! Actual reader rows and unchanged embedded-AST visibility.
use super::composition::LiteralExpressionParseItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::literal_expression_parsing::{
    interface::LiteralExpressionSyntax as _,
    types::{CarrierInput, LiteralDialect, LiteralParseOutcome},
};
use crate::domain::relations::collections::relations::{
    embedded_expression::item::EmbeddedExpressionItem, fixtures::contract::check,
    semantic_syntax::item::SemanticSyntaxItem,
};
use crate::fixtures::reading::{proposed, reading};
use crate::outbound::literal_expression::adapters::ruff_string_annotation::grammar::RuffStringAnnotation;
use ryo::languages::contract::{Reading, Want};

/// Extract the stable outcome field from each complete typed row.
fn outcomes(report: &Reading) -> Vec<&str> {
    report
        .rows
        .iter()
        .map(|row| match row {
            [_, _, Want::Name(outcome), _, _] => *outcome,
            _ => panic!("literal expression rows have an outcome"),
        })
        .collect()
}

fn rows_keep_empty_and_malformed_carriers_and_full_original_spans() {
    check::<LiteralExpressionParseItem>();
    let report = reading::<LiteralExpressionParseItem>("'T'\n''\n'T +'\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert_eq!(outcomes(&report), ["parsed", "malformed", "malformed"]);
    assert!(
        report
            .rows
            .iter()
            .all(|row| matches!(row, [_, Want::Name("python-string-annotation"), _, _, _]))
    );
    let spans = report
        .rows
        .iter()
        .map(|row| match row {
            [_, _, _, _, span] => *span,
            _ => panic!("five columns"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        spans,
        [
            ryo::languages::contract::want_span((1, 1), (1, 4)),
            ryo::languages::contract::want_span((2, 1), (2, 3)),
            ryo::languages::contract::want_span((3, 1), (3, 6)),
        ]
    );
}

fn actual_adapter_preserves_string_annotation_modes() {
    for (carrier, expected) in [
        ("'T'", LiteralParseOutcome::Parsed),
        ("r'T'", LiteralParseOutcome::Parsed),
        ("u'T'", LiteralParseOutcome::Parsed),
        ("''", LiteralParseOutcome::Malformed),
        ("'T +'", LiteralParseOutcome::Malformed),
        ("\"\"\"T\n| U\"\"\"", LiteralParseOutcome::Parsed),
        ("'T\\n| U'", LiteralParseOutcome::Malformed),
        ("('T\\n' '| U')", LiteralParseOutcome::Malformed),
        ("('T'\n'| U')", LiteralParseOutcome::Parsed),
        ("pass", LiteralParseOutcome::Unavailable),
        ("b'T'", LiteralParseOutcome::Unavailable),
        ("f'T'", LiteralParseOutcome::Unavailable),
        ("t'T'", LiteralParseOutcome::Unavailable),
    ] {
        assert_eq!(
            RuffStringAnnotation::parse(CarrierInput(carrier), LiteralDialect::PythonStringAnnotation),
            expected,
            "{carrier:?}",
        );
    }
}

fn token_reconstruction_preserves_unicode_comments_crlf_and_concatenation() {
    let report =
        reading::<LiteralExpressionParseItem>("(r'T' # trivia\r\n u'| U')\r\n'λ'\n'\\u03bb'\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert_eq!(outcomes(&report), ["parsed", "parsed", "parsed"]);
}

fn bytes_and_all_interpolated_and_template_strings_are_outside_the_relation() {
    let report = reading::<LiteralExpressionParseItem>("b'T'\nf'T'\nf'{value}'\nf'' 'T'\nt'T'\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert!(report.rows.is_empty());
}

fn additional_syntax_outcomes_do_not_publish_roots_or_arbitrary_names() {
    let source = "from typing import Literal\nx: Literal['arbitrary_metadata_name']\ny=list['unexposed_runtime_name']\n";
    let before = reading::<SemanticSyntaxItem>(source);
    let embedded = reading::<EmbeddedExpressionItem>(source);
    let report = reading::<LiteralExpressionParseItem>(source);
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert_eq!(outcomes(&report), ["parsed", "parsed"]);
    assert_eq!(reading::<SemanticSyntaxItem>(source), before);
    assert_eq!(reading::<EmbeddedExpressionItem>(source), embedded);
    assert_eq!(
        embedded.rows.len(),
        1,
        "the resolved builtin list is a real type context; Literal's metadata remains hidden"
    );
}

fn visible_embedded_literals_use_logical_spelling_after_physical_remapping() {
    let report = reading::<LiteralExpressionParseItem>("x: \"list['T']\"\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert!(
        report.rows.len() >= 2,
        "the original outer carrier and visible inner string are distinct occurrences"
    );
    assert!(outcomes(&report).iter().all(|outcome| *outcome == "parsed"));
}

fn incomplete_source_preserves_partial_knowledge() {
    let report = proposed::<LiteralExpressionParseItem>("'T'\n(\n");
    assert!(!report.causes.is_empty());
}

/// The cases of this file, one variant each.
#[derive(Clone, Copy)]
enum Body {
    /// The case `rows_keep_empty_and_malformed_carriers_and_full_original_spans`.
    RowsKeepEmptyAndMalformedCarriersAndFullOriginalSpans,
    /// The case `actual_adapter_preserves_string_annotation_modes`.
    ActualAdapterPreservesStringAnnotationModes,
    /// The case `token_reconstruction_preserves_unicode_comments_crlf_and_concatenation`.
    TokenReconstructionPreservesUnicodeCommentsCrlfAndConcatenation,
    /// The case `bytes_and_all_interpolated_and_template_strings_are_outside_the_relation`.
    BytesAndAllInterpolatedAndTemplateStringsAreOutsideTheRelation,
    /// The case `additional_syntax_outcomes_do_not_publish_roots_or_arbitrary_names`.
    AdditionalSyntaxOutcomesDoNotPublishRootsOrArbitraryNames,
    /// The case `visible_embedded_literals_use_logical_spelling_after_physical_remapping`.
    VisibleEmbeddedLiteralsUseLogicalSpellingAfterPhysicalRemapping,
    /// The case `incomplete_source_preserves_partial_knowledge`.
    IncompleteSourcePreservesPartialKnowledge,
}

type Input = Body;
type Value = ();
type Failure = ();

/// The rows of the cases whose bodies assert for themselves.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rows_keep_empty_and_malformed_carriers_and_full_original_spans"),
        word: Word::Succeeds,
        input: || Body::RowsKeepEmptyAndMalformedCarriersAndFullOriginalSpans,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("actual_adapter_preserves_string_annotation_modes"),
        word: Word::Succeeds,
        input: || Body::ActualAdapterPreservesStringAnnotationModes,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("token_reconstruction_preserves_unicode_comments_crlf_and_concatenation"),
        word: Word::Succeeds,
        input: || Body::TokenReconstructionPreservesUnicodeCommentsCrlfAndConcatenation,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("bytes_and_all_interpolated_and_template_strings_are_outside_the_relation"),
        word: Word::Succeeds,
        input: || Body::BytesAndAllInterpolatedAndTemplateStringsAreOutsideTheRelation,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("additional_syntax_outcomes_do_not_publish_roots_or_arbitrary_names"),
        word: Word::Succeeds,
        input: || Body::AdditionalSyntaxOutcomesDoNotPublishRootsOrArbitraryNames,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("visible_embedded_literals_use_logical_spelling_after_physical_remapping"),
        word: Word::Succeeds,
        input: || Body::VisibleEmbeddedLiteralsUseLogicalSpellingAfterPhysicalRemapping,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("incomplete_source_preserves_partial_knowledge"),
        word: Word::Succeeds,
        input: || Body::IncompleteSourcePreservesPartialKnowledge,
        expect: Expect::Value(()),
    },
];

/// Runs the body of one case.
fn subject(given: Input) -> Result<Value, Failure> {
    match given {
        Body::RowsKeepEmptyAndMalformedCarriersAndFullOriginalSpans => {
            rows_keep_empty_and_malformed_carriers_and_full_original_spans()
        }
        Body::ActualAdapterPreservesStringAnnotationModes => {
            actual_adapter_preserves_string_annotation_modes()
        }
        Body::TokenReconstructionPreservesUnicodeCommentsCrlfAndConcatenation => {
            token_reconstruction_preserves_unicode_comments_crlf_and_concatenation()
        }
        Body::BytesAndAllInterpolatedAndTemplateStringsAreOutsideTheRelation => {
            bytes_and_all_interpolated_and_template_strings_are_outside_the_relation()
        }
        Body::AdditionalSyntaxOutcomesDoNotPublishRootsOrArbitraryNames => {
            additional_syntax_outcomes_do_not_publish_roots_or_arbitrary_names()
        }
        Body::VisibleEmbeddedLiteralsUseLogicalSpellingAfterPhysicalRemapping => {
            visible_embedded_literals_use_logical_spelling_after_physical_remapping()
        }
        Body::IncompleteSourcePreservesPartialKnowledge => {
            incomplete_source_preserves_partial_knowledge()
        }
    }
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[cfg(test)]
mod demand_test;
#[cfg(test)]
mod preseal_test;
