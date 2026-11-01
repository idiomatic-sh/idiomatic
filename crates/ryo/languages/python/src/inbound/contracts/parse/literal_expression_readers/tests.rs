//! Actual reader rows and unchanged embedded-AST visibility.
use super::LiteralExpressionParseItem;
use crate::domain::literal_expression_parsing::{
    interface::LiteralExpressionSyntax as _,
    types::{LiteralDialect, LiteralParseOutcome},
};
use crate::domain::relations::collections::relations::{
    embedded_expression::item::EmbeddedExpressionItem, fixtures::contract::check,
    semantic_syntax::item::SemanticSyntaxItem,
};
use crate::fixtures::reading::{proposed, reading};
use crate::outbound::literal_expression::ruff_string_annotation::RuffStringAnnotation;
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

#[test]
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

#[test]
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
            RuffStringAnnotation::parse(carrier, LiteralDialect::PythonStringAnnotation),
            expected,
            "{carrier:?}",
        );
    }
}

#[test]
fn token_reconstruction_preserves_unicode_comments_crlf_and_concatenation() {
    let report =
        reading::<LiteralExpressionParseItem>("(r'T' # trivia\r\n u'| U')\r\n'λ'\n'\\u03bb'\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert_eq!(outcomes(&report), ["parsed", "parsed", "parsed"]);
}

#[test]
fn bytes_and_all_interpolated_and_template_strings_are_outside_the_relation() {
    let report = reading::<LiteralExpressionParseItem>("b'T'\nf'T'\nf'{value}'\nf'' 'T'\nt'T'\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert!(report.rows.is_empty());
}

#[test]
fn additional_syntax_outcomes_do_not_publish_roots_or_arbitrary_names() {
    let source = "from typing import Literal\nx: Literal['arbitrary_metadata_name']\ny=list['unexposed_runtime_name']\n";
    let before = reading::<SemanticSyntaxItem>(source);
    let embedded = reading::<EmbeddedExpressionItem>(source);
    let report = reading::<LiteralExpressionParseItem>(source);
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert_eq!(outcomes(&report), ["parsed", "parsed"]);
    assert_eq!(reading::<SemanticSyntaxItem>(source), before);
    assert_eq!(reading::<EmbeddedExpressionItem>(source), embedded);
    assert!(
        embedded.rows.is_empty(),
        "both candidates remain hidden or unrequested native strings"
    );
}

#[test]
fn visible_embedded_literals_use_logical_spelling_after_physical_remapping() {
    let report = reading::<LiteralExpressionParseItem>("x: \"list['T']\"\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert!(
        report.rows.len() >= 2,
        "the original outer carrier and visible inner string are distinct occurrences"
    );
    assert!(outcomes(&report).iter().all(|outcome| *outcome == "parsed"));
}

#[test]
fn incomplete_source_preserves_partial_knowledge() {
    let report = proposed::<LiteralExpressionParseItem>("'T'\n(\n");
    assert!(!report.causes.is_empty());
}
