//! Static raw-token reader binding; ordinary decoded format demand remains independent.
use super::format_readers::{error_parts, flag, intern, intern_character, summary_of, width_mode};
use crate::domain::format_syntax::service::flag_text;
use crate::domain::raw_format::constants::INCOMPLETE;
use crate::domain::{
    format_syntax::types::{FormatArgument, FormatParseResult, FormatParseState},
    model::types::Declarations,
    nodes::types::Node,
    raw_format::service::{prepare, result, segments, state},
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
                given error_character = intern_character(symbols, tree.source, errors.1);
                given segment_ordinal = languages::Integer(i64::try_from(segment.ordinal.0).unwrap_or(i64::MAX));
                given positional_consumption = languages::Integer(i64::try_from(summary.0.positional_consumption.0).unwrap_or(i64::MAX));
                row { id: node(occurrence.node), segment: node(segment.token), segment_ordinal: value(segment_ordinal), dialect: value(dialect), outcome: value(outcome), error_category: value(error_category), error_character: value(error_character), error_index: value(languages::Integer(errors.2)), positional_consumption: value(positional_consumption), mapping_present: value(flag(summary.0.mapping_present.into())), dynamic_width: value(flag(summary.0.dynamic_width.into())), dynamic_precision: value(flag(summary.0.dynamic_precision.into())), file: file, span: span(segment.token), }
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
            require bool::from(occurrence.complete);
            each (segments(&index, occurrence)) as segment {
                when result(&index, segment) {
                    Some(FormatParseResult::Parsed(parsed)) => {
                        each (parsed.fields.iter()) as field {
                            given argument_kind = intern(symbols, tree.source, match &field.argument { FormatArgument::Positional => "positional", FormatArgument::Mapping(_) => "mapping" });
                            given mapping_key = intern(symbols, tree.source, match &field.argument { FormatArgument::Positional => "", FormatArgument::Mapping(key) => key.0.as_str() });
                            given conversion = intern_character(symbols, tree.source, Some(field.conversion.0));
                            given flags = intern(symbols, tree.source, flag_text(field.flags).0);
                            given width = intern(symbols, tree.source, width_mode(field.width));
                            given precision = intern(symbols, tree.source, width_mode(field.precision));
                            given segment_ordinal = languages::Integer(i64::try_from(segment.ordinal.0).unwrap_or(i64::MAX));
                            given ordinal = languages::Integer(i64::try_from(segment.first_field.0 + field.ordinal.0).unwrap_or(i64::MAX));
                            given local = languages::Integer(i64::try_from(field.ordinal.0).unwrap_or(i64::MAX));
                            row { id: node(occurrence.node), segment: node(segment.token), segment_ordinal: value(segment_ordinal), ordinal: value(ordinal), segment_field_ordinal: value(local), argument_kind: value(argument_kind), mapping_key: value(mapping_key), conversion: value(conversion), width_mode: value(width), precision_mode: value(precision), file: file, span: span(segment.token), flags: value(flags), }
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
#[path = "raw_format_readers_test.rs"]
mod tests;
