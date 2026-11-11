//! Actual reader rows and unchanged embedded-AST visibility.
use super::composition::LiteralExpressionParseItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::literal_expression_parsing::types::{CarrierInput, LiteralDialect};
use crate::domain::relations::collections::relations::{
    embedded_expression::item::EmbeddedExpressionItem, fixtures::contract::check,
    semantic_syntax::item::SemanticSyntaxItem,
};
use crate::fixtures::reading::{proposed, reading};
use crate::outbound::literal_expression::adapters::ruff_string_annotation::grammar::RuffStringAnnotation;
use crate::outbound::literal_expression::interface::LiteralExpressionSyntaxPort as _;
use ryo::languages::contract::{Reading, Want};

/// The outcome cell of each row; a row of another shape shows as `<not five columns>`.
fn outcomes(report: &Reading) -> Vec<&str> {
    report
        .rows
        .iter()
        .map(|row| match row {
            [_, _, Want::Name(outcome), _, _] => *outcome,
            _ => "<not five columns>",
        })
        .collect()
}

/// The dialect cell of each row.
fn dialects(report: &Reading) -> Vec<String> {
    report
        .rows
        .iter()
        .map(|row| match row {
            [_, dialect, _, _, _] => format!("{dialect:?}"),
            _ => "<not five columns>".to_owned(),
        })
        .collect()
}

/// The span cell of each row.
fn spans(report: &Reading) -> Vec<String> {
    report
        .rows
        .iter()
        .map(|row| match row {
            [_, _, _, _, span] => format!("{span:?}"),
            _ => "<not five columns>".to_owned(),
        })
        .collect()
}

/// The causes and the outcomes of a reading, as a row states them.
fn shown(report: &Reading) -> String {
    format!(
        "causes {:?}; outcomes {:?}",
        report.causes,
        outcomes(report)
    )
}

fn rows_keep_empty_and_malformed_carriers_and_full_original_spans() -> String {
    check::<LiteralExpressionParseItem>();
    let report = reading::<LiteralExpressionParseItem>("'T'\n''\n'T +'\n");
    format!(
        "{}; dialects {:?}; spans {:?}",
        shown(&report),
        dialects(&report),
        spans(&report)
    )
}

fn actual_adapter_preserves_string_annotation_modes() -> String {
    [
        "'T'",
        "r'T'",
        "u'T'",
        "''",
        "'T +'",
        "\"\"\"T\n| U\"\"\"",
        "'T\\n| U'",
        "('T\\n' '| U')",
        "('T'\n'| U')",
        "pass",
        "b'T'",
        "f'T'",
        "t'T'",
    ]
    .iter()
    .map(|carrier| {
        format!(
            "{carrier:?} {:?}",
            RuffStringAnnotation.parse(
                CarrierInput(carrier),
                LiteralDialect::PythonStringAnnotation
            )
        )
    })
    .collect::<Vec<_>>()
    .join("; ")
}

fn token_reconstruction_preserves_unicode_comments_crlf_and_concatenation() -> String {
    shown(&reading::<LiteralExpressionParseItem>(
        "(r'T' # trivia\r\n u'| U')\r\n'λ'\n'\\u03bb'\n",
    ))
}

fn bytes_and_all_interpolated_and_template_strings_are_outside_the_relation() -> String {
    let report = reading::<LiteralExpressionParseItem>("b'T'\nf'T'\nf'{value}'\nf'' 'T'\nt'T'\n");
    format!("causes {:?}; rows {}", report.causes, report.rows.len())
}

fn additional_syntax_outcomes_do_not_publish_roots_or_arbitrary_names() -> String {
    let source = "from typing import Literal\nx: Literal['arbitrary_metadata_name']\ny=list['unexposed_runtime_name']\n";
    let before = reading::<SemanticSyntaxItem>(source);
    let embedded = reading::<EmbeddedExpressionItem>(source);
    let report = reading::<LiteralExpressionParseItem>(source);
    // The resolved builtin list is a real type context; Literal's metadata remains hidden.
    format!(
        "{}; semantic syntax unchanged {}; embedded unchanged {}; embedded rows {}",
        shown(&report),
        reading::<SemanticSyntaxItem>(source) == before,
        reading::<EmbeddedExpressionItem>(source) == embedded,
        embedded.rows.len()
    )
}

fn visible_embedded_literals_use_logical_spelling_after_physical_remapping() -> String {
    // The original outer carrier and the visible inner string are distinct occurrences.
    shown(&reading::<LiteralExpressionParseItem>("x: \"list['T']\"\n"))
}

fn incomplete_source_preserves_partial_knowledge() -> String {
    format!(
        "causes {:?}",
        proposed::<LiteralExpressionParseItem>("'T'\n(\n").causes
    )
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
/// What the case observed, rendered.
type Value = &'static str;
type Failure = ();

/// The rows of this file's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rows_keep_empty_and_malformed_carriers_and_full_original_spans"),
        word: Word::Succeeds,
        input: || Body::RowsKeepEmptyAndMalformedCarriersAndFullOriginalSpans,
        expect: Expect::Value(
            "causes []; outcomes [\"parsed\", \"malformed\", \"malformed\"]; dialects [\"Name(\\\"python-string-annotation\\\")\", \"Name(\\\"python-string-annotation\\\")\", \"Name(\\\"python-string-annotation\\\")\"]; spans [\"Span(Point { line: 1, column: 1 }, Point { line: 1, column: 4 })\", \"Span(Point { line: 2, column: 1 }, Point { line: 2, column: 3 })\", \"Span(Point { line: 3, column: 1 }, Point { line: 3, column: 6 })\"]",
        ),
    },
    Case {
        name: CaseName("actual_adapter_preserves_string_annotation_modes"),
        word: Word::Succeeds,
        input: || Body::ActualAdapterPreservesStringAnnotationModes,
        expect: Expect::Value(
            "\"'T'\" Parsed; \"r'T'\" Parsed; \"u'T'\" Parsed; \"''\" Malformed; \"'T +'\" Malformed; \"\\\"\\\"\\\"T\\n| U\\\"\\\"\\\"\" Parsed; \"'T\\\\n| U'\" Malformed; \"('T\\\\n' '| U')\" Malformed; \"('T'\\n'| U')\" Parsed; \"pass\" Unavailable; \"b'T'\" Unavailable; \"f'T'\" Unavailable; \"t'T'\" Unavailable",
        ),
    },
    Case {
        name: CaseName("token_reconstruction_preserves_unicode_comments_crlf_and_concatenation"),
        word: Word::Succeeds,
        input: || Body::TokenReconstructionPreservesUnicodeCommentsCrlfAndConcatenation,
        expect: Expect::Value("causes []; outcomes [\"parsed\", \"parsed\", \"parsed\"]"),
    },
    Case {
        name: CaseName("bytes_and_all_interpolated_and_template_strings_are_outside_the_relation"),
        word: Word::Succeeds,
        input: || Body::BytesAndAllInterpolatedAndTemplateStringsAreOutsideTheRelation,
        expect: Expect::Value("causes []; rows 0"),
    },
    Case {
        name: CaseName("additional_syntax_outcomes_do_not_publish_roots_or_arbitrary_names"),
        word: Word::Succeeds,
        input: || Body::AdditionalSyntaxOutcomesDoNotPublishRootsOrArbitraryNames,
        expect: Expect::Value(
            "causes []; outcomes [\"parsed\", \"parsed\"]; semantic syntax unchanged true; embedded unchanged true; embedded rows 1",
        ),
    },
    Case {
        name: CaseName("visible_embedded_literals_use_logical_spelling_after_physical_remapping"),
        word: Word::Succeeds,
        input: || Body::VisibleEmbeddedLiteralsUseLogicalSpellingAfterPhysicalRemapping,
        expect: Expect::Value("causes []; outcomes [\"parsed\", \"parsed\"]"),
    },
    Case {
        name: CaseName("incomplete_source_preserves_partial_knowledge"),
        word: Word::Succeeds,
        input: || Body::IncompleteSourcePreservesPartialKnowledge,
        expect: Expect::Value(
            "causes [\"literal-expression parsing requires complete logical string carriers\"]",
        ),
    },
];

/// Runs the body of one case.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
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
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[cfg(test)]
mod demand_test;
#[cfg(test)]
mod preseal_test;
