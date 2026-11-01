//! Static raw-token reader binding; ordinary decoded format demand remains independent.
use super::format_readers::{error_parts, flag, intern, summary_of, width_mode};
use crate::domain::{
    format_syntax::types::{FormatArgument, FormatParseResult, FormatParseState},
    model::types::Declarations,
    nodes::types::Node,
    raw_format::service::{INCOMPLETE, prepare, result, segments, state},
    syntax::{service::kind_bits, types::Kind},
};
use crate::outbound::format_syntax::adapters::ruff_literal_cformat::RuffLiteralCFormat;
use ryo::languages;

/// Reads one parse outcome for each original raw string token.
pub(super) struct RawFormatSegmentItem;
/// Reads fields parsed independently from each original raw string token.
pub(super) struct RawFormatFieldItem;

ryo::reads! {
    RawFormatSegmentItem => languages::RawFormatSegment, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        given index = prepare::<RuffLiteralCFormat>(tree, model, symbols);
        each (index.occurrences.iter()) as occurrence {
            each (segments(&index, occurrence)) as segment {
                given parsed = result(&index, segment);
                given summary = summary_of(parsed, false);
                given errors = error_parts(parsed);
                given dialect = intern(symbols, tree.source, "percent-string");
                given outcome = intern(symbols, tree.source, summary.1);
                given error_category = intern(symbols, tree.source, errors.0);
                given error_character = intern(symbols, tree.source, &errors.1);
                given segment_ordinal = languages::Integer(i64::try_from(segment.ordinal).unwrap_or(i64::MAX));
                given positional_consumption = languages::Integer(i64::try_from(summary.0.positional_consumption).unwrap_or(i64::MAX));
                row { id: node(occurrence.node), segment: node(segment.token), segment_ordinal: value(segment_ordinal), dialect: value(dialect), outcome: value(outcome), error_category: value(error_category), error_character: value(error_character), error_index: value(languages::Integer(errors.2)), positional_consumption: value(positional_consumption), mapping_present: value(flag(summary.0.mapping_present)), dynamic_width: value(flag(summary.0.dynamic_width)), dynamic_precision: value(flag(summary.0.dynamic_precision)), file: file, span: span(segment.token), }
            }
        }
        when state(&index) {
            FormatParseState::Fresh | FormatParseState::Partial => { partial INCOMPLETE => languages::Missing::Any; }
            FormatParseState::Complete => {}
        }
    }
}

ryo::reads! {
    RawFormatFieldItem => languages::RawFormatField, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        given index = prepare::<RuffLiteralCFormat>(tree, model, symbols);
        each (index.occurrences.iter()) as occurrence {
            require occurrence.complete;
            each (segments(&index, occurrence)) as segment {
                when result(&index, segment) {
                    Some(FormatParseResult::Parsed(parsed)) => {
                        each (parsed.fields.iter()) as field {
                            given argument_kind = intern(symbols, tree.source, match &field.argument { FormatArgument::Positional => "positional", FormatArgument::Mapping(_) => "mapping" });
                            given mapping_key = intern(symbols, tree.source, match &field.argument { FormatArgument::Positional => "", FormatArgument::Mapping(key) => key });
                            given conversion = intern(symbols, tree.source, &field.conversion.to_string());
                            given width = intern(symbols, tree.source, width_mode(field.width));
                            given precision = intern(symbols, tree.source, width_mode(field.precision));
                            given segment_ordinal = languages::Integer(i64::try_from(segment.ordinal).unwrap_or(i64::MAX));
                            given ordinal = languages::Integer(i64::try_from(segment.first_field + field.ordinal).unwrap_or(i64::MAX));
                            given local = languages::Integer(i64::try_from(field.ordinal).unwrap_or(i64::MAX));
                            row { id: node(occurrence.node), segment: node(segment.token), segment_ordinal: value(segment_ordinal), ordinal: value(ordinal), segment_field_ordinal: value(local), argument_kind: value(argument_kind), mapping_key: value(mapping_key), conversion: value(conversion), width_mode: value(width), precision_mode: value(precision), file: file, span: span(segment.token), }
                        }
                    }
                    Some(FormatParseResult::Malformed(_) | FormatParseResult::Unavailable) => {}
                    None => {}
                }
            }
        }
        when state(&index) {
            FormatParseState::Fresh | FormatParseState::Partial => { partial INCOMPLETE => languages::Missing::Any; }
            FormatParseState::Complete => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RawFormatFieldItem, RawFormatSegmentItem};
    use crate::domain::relations::collections::relations::fixtures::contract::check;
    use crate::fixtures::reading::{proposed, reading};
    use ryo::languages::contract::Want;

    #[test]
    fn raw_format_rows_keep_empty_segments_repeated_keys_and_star_conversions() {
        check::<RawFormatSegmentItem>();
        check::<RawFormatFieldItem>();
        let source = "'' '%%' '%(x)d %(x)s %*.*d'\n";
        let segments = reading::<RawFormatSegmentItem>(source);
        assert!(segments.causes.is_empty(), "{:?}", segments.causes);
        assert_eq!(segments.rows.len(), 3);
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
            let expected_index = i64::try_from(at).unwrap_or(i64::MAX);
            assert_eq!(row[2], Want::Number(2));
            assert_eq!(row[3], Want::Number(expected_index));
            assert_eq!(row[4], Want::Number(expected_index));
        }
        assert_eq!(fields.rows[0][6], Want::Name("x"));
        assert_eq!(fields.rows[1][6], Want::Name("x"));
        assert_eq!(fields.rows[2][8], Want::Name("star"));
        assert_eq!(fields.rows[2][9], Want::Name("star"));
    }

    #[test]
    fn raw_format_failed_segments_contribute_no_valid_prefix_fields() {
        let source = "'%s' '%d%q' '%x'\n";
        let segments = reading::<RawFormatSegmentItem>(source);
        assert!(segments.causes.is_empty());
        assert_eq!(segments.rows.len(), 3);
        assert_eq!(segments.rows[1][4], Want::Name("malformed"));
        assert_eq!(segments.rows[1][6], Want::Name("q"));
        assert_eq!(segments.rows[1][7], Want::Number(3));
        let fields = reading::<RawFormatFieldItem>(source);
        assert!(fields.causes.is_empty());
        assert_eq!(fields.rows.len(), 2);
        assert_eq!(fields.rows[0][3], Want::Number(0));
        assert_eq!(fields.rows[1][2], Want::Number(2));
        assert_eq!(fields.rows[1][3], Want::Number(1));
        assert_eq!(fields.rows[1][4], Want::Number(0));
        assert_eq!(fields.rows[1][7], Want::Name("x"));
    }

    #[test]
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
        assert_eq!(fields.rows[0][2], Want::Number(1));
        assert_eq!(fields.rows[0][3], Want::Number(0));
        assert_eq!(fields.rows[1][6], Want::Name(r"\x78"));
    }

    #[test]
    fn raw_format_accepts_prefixes_quotes_comments_and_original_crlf() {
        let segments =
            reading::<RawFormatSegmentItem>("(r'%d' # keep\r\n U\"%s\" '''%c\r\n%x''')\r\n");
        assert!(segments.causes.is_empty(), "{:?}", segments.causes);
        assert_eq!(segments.rows.len(), 3);
        assert_eq!(segments.rows[0][8], Want::Number(1));
        assert_eq!(segments.rows[1][8], Want::Number(1));
        assert_eq!(segments.rows[2][8], Want::Number(2));
    }

    #[test]
    fn raw_format_uses_embedded_logical_spelling_and_excludes_hidden_candidates() {
        let source = "from typing import Literal\nx: \"Literal[\\\"%d\\\"]\"\ny: \"Literal['%(\\\\x78)d']\"\n";
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
        let segments = reading::<RawFormatSegmentItem>(hidden);
        assert!(segments.causes.is_empty(), "{:?}", segments.causes);
        assert_eq!(
            segments.rows.len(),
            1,
            "value-only embedded candidates are hidden, while the native literal remains visible"
        );
    }

    #[test]
    fn raw_format_excludes_bytes_interpolated_and_template_strings() {
        let source = "b'%d'\nf'%d'\nf'%d {x}'\nt'%d'\n";
        let segments = reading::<RawFormatSegmentItem>(source);
        assert!(segments.causes.is_empty(), "{:?}", segments.causes);
        assert!(segments.rows.is_empty());
        let fields = reading::<RawFormatFieldItem>(source);
        assert!(fields.causes.is_empty());
        assert!(fields.rows.is_empty());
    }

    #[test]
    fn raw_format_incomplete_syntax_is_partial_and_has_no_precise_global_fields() {
        let segments = proposed::<RawFormatSegmentItem>("'%d'\n(\n");
        assert!(!segments.causes.is_empty());
        let fields = proposed::<RawFormatFieldItem>("'%d'\n(\n");
        assert!(!fields.causes.is_empty());
        assert!(fields.rows.is_empty());
    }
}
