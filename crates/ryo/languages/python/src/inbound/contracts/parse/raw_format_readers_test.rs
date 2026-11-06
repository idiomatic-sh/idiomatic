//! Regression cases for raw format readers bindings.
use super::{RawFormatFieldItem, RawFormatSegmentItem};
use crate::domain::relations::collections::relations::fixtures::contract::check;
use crate::fixtures::reading::{proposed, reading};
use ryo::languages::contract::Want;
use crate::cases::{Case, CaseName, Expect, Word, run};

fn raw_format_rows_keep_empty_segments_repeated_keys_and_star_conversions() {
    check::<RawFormatSegmentItem>();
    check::<RawFormatFieldItem>();
    let source = "'' '%%' '%(x)d %(x)s %*.*d'\n";
    let segments = reading::<RawFormatSegmentItem>(source);
    assert!(segments.causes.is_empty(), "{:?}", segments.causes);
    assert_eq!(segments.rows.len(), 3);
    assert!(segments.rows[0].len() > 8);
    assert!(segments.rows[2].len() > 11);
    assert_eq!(segments.rows[0][8], Want::Number(0));
    assert_eq!(segments.rows[1][8], Want::Number(0));
    assert_eq!(segments.rows[2][8], Want::Number(3));
    assert_eq!(segments.rows[2][9], Want::Flag(true));
    assert_eq!(segments.rows[2][10], Want::Flag(true));
    assert_eq!(segments.rows[2][11], Want::Flag(true));
    assert_eq!(segments.rows[0][0], segments.rows[2][0]);
    assert_ne!(segments.rows[0][1], segments.rows[2][1]);
    let fields = reading::<RawFormatFieldItem>(source);
    assert!(fields.causes.is_empty());
    assert_eq!(fields.rows.len(), 3);
    for (at, row) in fields.rows.iter().enumerate() {
        assert!(row.len() > 4);
        let expected_index = i64::try_from(at).unwrap_or(i64::MAX);
        assert_eq!(row[2], Want::Number(2));
        assert_eq!(row[3], Want::Number(expected_index));
        assert_eq!(row[4], Want::Number(expected_index));
    }
    assert_eq!(fields.rows[0][6], Want::Name("x"));
    assert_eq!(fields.rows[1][6], Want::Name("x"));
    assert!(fields.rows[2].len() > 9);
    assert_eq!(fields.rows[2][8], Want::Name("star"));
    assert_eq!(fields.rows[2][9], Want::Name("star"));
}

fn raw_format_failed_segments_contribute_no_valid_prefix_fields() {
    let source = "'%s' '%d%q' '%x'\n";
    let segments = reading::<RawFormatSegmentItem>(source);
    assert!(segments.causes.is_empty());
    assert_eq!(segments.rows.len(), 3);
    assert!(segments.rows[1].len() > 7);
    assert_eq!(segments.rows[1][4], Want::Name("malformed"));
    assert_eq!(segments.rows[1][6], Want::Name("q"));
    assert_eq!(segments.rows[1][7], Want::Number(3));
    let fields = reading::<RawFormatFieldItem>(source);
    assert!(fields.causes.is_empty());
    assert_eq!(fields.rows.len(), 2);
    assert!(fields.rows[1].len() > 7);
    assert_eq!(fields.rows[0][3], Want::Number(0));
    assert_eq!(fields.rows[1][2], Want::Number(2));
    assert_eq!(fields.rows[1][3], Want::Number(1));
    assert_eq!(fields.rows[1][4], Want::Number(0));
    assert_eq!(fields.rows[1][7], Want::Name("x"));
}

fn raw_format_preserves_escapes_segment_boundaries_and_unicode_indices() {
    let source = "'\\x25d' '%d'\n'%' 'd'\n'é%n'\n'%(\\x78)d'\n";
    let segments = reading::<RawFormatSegmentItem>(source);
    assert!(segments.causes.is_empty());
    assert_eq!(segments.rows.len(), 6);
    assert_eq!(segments.rows[0][8], Want::Number(0));
    assert_eq!(segments.rows[1][8], Want::Number(1));
    assert_eq!(segments.rows[2][4], Want::Name("malformed"));
    assert_eq!(segments.rows[3][4], Want::Name("parsed"));
    assert_eq!(segments.rows[4][7], Want::Number(2));
    let fields = reading::<RawFormatFieldItem>(source);
    assert!(fields.causes.is_empty());
    assert_eq!(fields.rows.len(), 2);
    assert!(fields.rows[0].len() > 3);
    assert_eq!(fields.rows[0][2], Want::Number(1));
    assert_eq!(fields.rows[0][3], Want::Number(0));
    assert_eq!(fields.rows[1][6], Want::Name(r"\x78"));
}

fn raw_format_accepts_prefixes_quotes_comments_and_original_crlf() {
    let segments = reading::<RawFormatSegmentItem>("(r'%d' # keep\r\n U\"%s\" '''%c\r\n%x''')\r\n");
    assert!(segments.causes.is_empty(), "{:?}", segments.causes);
    assert_eq!(segments.rows.len(), 3);
    assert_eq!(segments.rows[0][8], Want::Number(1));
    assert_eq!(segments.rows[1][8], Want::Number(1));
    assert_eq!(segments.rows[2][8], Want::Number(2));
}

fn raw_format_uses_embedded_logical_spelling_and_excludes_hidden_candidates() {
    let source =
        "from typing import Literal\nx: \"Literal[\\\"%d\\\"]\"\ny: \"Literal['%(\\\\x78)d']\"\n";
    let segments = reading::<RawFormatSegmentItem>(source);
    assert!(segments.causes.is_empty(), "{:?}", segments.causes);
    assert_eq!(
        segments.rows.len(),
        4,
        "native carriers and enabled embedded literals retain distinct identities"
    );
    assert!(
        segments
            .rows
            .iter()
            .all(|row| row[4] == Want::Name("parsed"))
    );
    let fields = reading::<RawFormatFieldItem>(source);
    assert!(fields.causes.is_empty(), "{:?}", fields.causes);
    assert_eq!(fields.rows.len(), 4);
    assert_eq!(fields.rows[0][7], Want::Name("d"));
    assert!(
        fields.rows.iter().any(|row| row[6] == Want::Name(r"\x78")),
        "embedded token uses logical spelling"
    );
    assert!(
        fields.rows.iter().any(|row| row[6] == Want::Name(r"\\x78")),
        "native carrier keeps its own spelling"
    );
    let hidden = "from typing import Literal\nx: Literal[\"list['%d']\"]\n";
    let hidden_segments = reading::<RawFormatSegmentItem>(hidden);
    assert!(
        hidden_segments.causes.is_empty(),
        "{:?}",
        hidden_segments.causes
    );
    assert_eq!(
        hidden_segments.rows.len(),
        1,
        "value-only embedded candidates are hidden, while the native literal remains visible"
    );
}

fn raw_format_excludes_bytes_interpolated_and_template_strings() {
    let source = "b'%d'\nf'%d'\nf'%d {x}'\nt'%d'\n";
    let segments = reading::<RawFormatSegmentItem>(source);
    assert!(segments.causes.is_empty(), "{:?}", segments.causes);
    assert!(segments.rows.is_empty());
    let fields = reading::<RawFormatFieldItem>(source);
    assert!(fields.causes.is_empty());
    assert!(fields.rows.is_empty());
}

fn raw_format_incomplete_syntax_is_partial_and_has_no_precise_global_fields() {
    let segments = proposed::<RawFormatSegmentItem>("'%d'\n(\n");
    assert!(!segments.causes.is_empty());
    let fields = proposed::<RawFormatFieldItem>("'%d'\n(\n");
    assert!(!fields.causes.is_empty());
    assert!(fields.rows.is_empty());
}

fn raw_error_characters_preserve_two_and_four_byte_unicode_scalars() {
    let report = reading::<RawFormatSegmentItem>("a = '%é'\nb = '%🧪'\nc = '%s'\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert_eq!(report.rows.len(), 3);
    assert!(report.rows[0].len() > 7);
    assert!(report.rows[1].len() > 7);
    assert_eq!(report.rows[0][6], Want::Name("é"));
    assert_eq!(report.rows[1][6], Want::Name("🧪"));
    assert_eq!(report.rows[2][6], Want::Name(""));
    assert_eq!(report.rows[0][7], Want::Number(1));
    assert_eq!(report.rows[1][7], Want::Number(1));
    let fields = reading::<RawFormatFieldItem>("a = '%s'\n");
    assert_eq!(fields.rows[0][7], Want::Name("s"));
}

fn raw_flags_append_to_the_schema_and_preserve_normalized_parse_metadata() {
    let report = reading::<RawFormatFieldItem>("'%s' '%0- +#10r' '%##00--++  r' '%0s' '%%'\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert_eq!(report.rows.len(), 4);
    assert!(report.rows.iter().all(|row| row.len() == 13));
    assert_eq!(report.rows[0][7], Want::Name("s"));
    assert_eq!(report.rows[0][12], Want::Name(""));
    assert_eq!(report.rows[1][8], Want::Name("fixed"));
    assert_eq!(report.rows[1][12], Want::Name(" #+-0"));
    assert_eq!(report.rows[2][8], Want::Name("none"));
    assert_eq!(report.rows[2][12], Want::Name(" #+-0"));
    assert_eq!(report.rows[3][8], Want::Name("none"));
    assert_eq!(report.rows[3][12], Want::Name("0"));
}

fn raw_flags_do_not_decode_escaped_flags_or_invent_fields_from_percent_escapes() {
    let report = reading::<RawFormatFieldItem>("'\\x25+r' '%\\x2br' '%%' '%+r'\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0][2], Want::Number(3));
    assert_eq!(report.rows[0][7], Want::Name("r"));
    assert_eq!(report.rows[0][12], Want::Name("+"));
}

/// Which of the file's cases a row runs.
#[derive(Clone, Copy, Debug)]
enum Body {
    /// The case `raw_format_rows_keep_empty_segments_repeated_keys_and_star_conversions`.
    RawFormatRowsKeepEmptySegmentsRepeatedKeysAndStarConversions,
    /// The case `raw_format_failed_segments_contribute_no_valid_prefix_fields`.
    RawFormatFailedSegmentsContributeNoValidPrefixFields,
    /// The case `raw_format_preserves_escapes_segment_boundaries_and_unicode_indices`.
    RawFormatPreservesEscapesSegmentBoundariesAndUnicodeIndices,
    /// The case `raw_format_accepts_prefixes_quotes_comments_and_original_crlf`.
    RawFormatAcceptsPrefixesQuotesCommentsAndOriginalCrlf,
    /// The case `raw_format_uses_embedded_logical_spelling_and_excludes_hidden_candidates`.
    RawFormatUsesEmbeddedLogicalSpellingAndExcludesHiddenCandidates,
    /// The case `raw_format_excludes_bytes_interpolated_and_template_strings`.
    RawFormatExcludesBytesInterpolatedAndTemplateStrings,
    /// The case `raw_format_incomplete_syntax_is_partial_and_has_no_precise_global_fields`.
    RawFormatIncompleteSyntaxIsPartialAndHasNoPreciseGlobalFields,
    /// The case `raw_error_characters_preserve_two_and_four_byte_unicode_scalars`.
    RawErrorCharactersPreserveTwoAndFourByteUnicodeScalars,
    /// The case `raw_flags_append_to_the_schema_and_preserve_normalized_parse_metadata`.
    RawFlagsAppendToTheSchemaAndPreserveNormalizedParseMetadata,
    /// The case `raw_flags_do_not_decode_escaped_flags_or_invent_fields_from_percent_escapes`.
    RawFlagsDoNotDecodeEscapedFlagsOrInventFieldsFromPercentEscapes,
}

/// The rows of the cases whose bodies assert for themselves.
const CASES: &[Case<Body, (), ()>] = &[
    Case {
        name: CaseName("raw_format_rows_keep_empty_segments_repeated_keys_and_star_conversions"),
        word: Word::Succeeds,
        input: || Body::RawFormatRowsKeepEmptySegmentsRepeatedKeysAndStarConversions,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("raw_format_failed_segments_contribute_no_valid_prefix_fields"),
        word: Word::Succeeds,
        input: || Body::RawFormatFailedSegmentsContributeNoValidPrefixFields,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("raw_format_preserves_escapes_segment_boundaries_and_unicode_indices"),
        word: Word::Succeeds,
        input: || Body::RawFormatPreservesEscapesSegmentBoundariesAndUnicodeIndices,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("raw_format_accepts_prefixes_quotes_comments_and_original_crlf"),
        word: Word::Succeeds,
        input: || Body::RawFormatAcceptsPrefixesQuotesCommentsAndOriginalCrlf,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("raw_format_uses_embedded_logical_spelling_and_excludes_hidden_candidates"),
        word: Word::Succeeds,
        input: || Body::RawFormatUsesEmbeddedLogicalSpellingAndExcludesHiddenCandidates,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("raw_format_excludes_bytes_interpolated_and_template_strings"),
        word: Word::Succeeds,
        input: || Body::RawFormatExcludesBytesInterpolatedAndTemplateStrings,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("raw_format_incomplete_syntax_is_partial_and_has_no_precise_global_fields"),
        word: Word::Succeeds,
        input: || Body::RawFormatIncompleteSyntaxIsPartialAndHasNoPreciseGlobalFields,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("raw_error_characters_preserve_two_and_four_byte_unicode_scalars"),
        word: Word::Succeeds,
        input: || Body::RawErrorCharactersPreserveTwoAndFourByteUnicodeScalars,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("raw_flags_append_to_the_schema_and_preserve_normalized_parse_metadata"),
        word: Word::Succeeds,
        input: || Body::RawFlagsAppendToTheSchemaAndPreserveNormalizedParseMetadata,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("raw_flags_do_not_decode_escaped_flags_or_invent_fields_from_percent_escapes"),
        word: Word::Succeeds,
        input: || Body::RawFlagsDoNotDecodeEscapedFlagsOrInventFieldsFromPercentEscapes,
        expect: Expect::Value(()),
    },
];

/// Runs the body of one case.
fn body(given: Body) -> Result<(), ()> {
    match given {
        Body::RawFormatRowsKeepEmptySegmentsRepeatedKeysAndStarConversions => raw_format_rows_keep_empty_segments_repeated_keys_and_star_conversions(),
        Body::RawFormatFailedSegmentsContributeNoValidPrefixFields => raw_format_failed_segments_contribute_no_valid_prefix_fields(),
        Body::RawFormatPreservesEscapesSegmentBoundariesAndUnicodeIndices => raw_format_preserves_escapes_segment_boundaries_and_unicode_indices(),
        Body::RawFormatAcceptsPrefixesQuotesCommentsAndOriginalCrlf => raw_format_accepts_prefixes_quotes_comments_and_original_crlf(),
        Body::RawFormatUsesEmbeddedLogicalSpellingAndExcludesHiddenCandidates => raw_format_uses_embedded_logical_spelling_and_excludes_hidden_candidates(),
        Body::RawFormatExcludesBytesInterpolatedAndTemplateStrings => raw_format_excludes_bytes_interpolated_and_template_strings(),
        Body::RawFormatIncompleteSyntaxIsPartialAndHasNoPreciseGlobalFields => raw_format_incomplete_syntax_is_partial_and_has_no_precise_global_fields(),
        Body::RawErrorCharactersPreserveTwoAndFourByteUnicodeScalars => raw_error_characters_preserve_two_and_four_byte_unicode_scalars(),
        Body::RawFlagsAppendToTheSchemaAndPreserveNormalizedParseMetadata => raw_flags_append_to_the_schema_and_preserve_normalized_parse_metadata(),
        Body::RawFlagsDoNotDecodeEscapedFlagsOrInventFieldsFromPercentEscapes => raw_flags_do_not_decode_escaped_flags_or_invent_fields_from_percent_escapes(),
    }
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, body);
}
