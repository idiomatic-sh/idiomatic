//! Static binding from the Python composition to its percent-format grammar adapter.

use crate::domain::format_parsing::service::{INCOMPLETE, occurrences, prepare, result, state};
use crate::domain::format_syntax::types::{
    FormatArgument, FormatErrorKind, FormatParseResult, FormatParseState, FormatQuantityMode,
    FormatSummary,
};
use crate::domain::{
    model::types::Declarations, nodes::types::Node, semantic_syntax::types::Visibility,
    syntax::service::kind_bits, syntax::types::Kind,
};
use crate::outbound::format_syntax::adapters::ruff_literal_cformat::RuffLiteralCFormat;
use ryo::languages;

pub(super) struct FormatParseItem;
pub(super) struct FormatFieldItem;

fn intern(
    symbols: &mut languages::Symbols,
    source: languages::ByteSlice<'_>,
    value: &str,
) -> languages::Symbol {
    languages::intern_derived(symbols, source, languages::NameText(value))
}

fn summary_of(result: Option<&FormatParseResult>, unknown: bool) -> (FormatSummary, &'static str) {
    match result {
        Some(FormatParseResult::Parsed(parsed)) => (parsed.summary, "parsed"),
        Some(FormatParseResult::Malformed(_)) => (FormatSummary::default(), "malformed"),
        Some(FormatParseResult::Unavailable) => (FormatSummary::default(), "unavailable"),
        None if unknown => (FormatSummary::default(), "unknown-input"),
        None => (FormatSummary::default(), "unavailable"),
    }
}

fn error_parts(result: Option<&FormatParseResult>) -> (&'static str, String, i64) {
    let Some(FormatParseResult::Malformed(error)) = result else {
        return ("none", String::new(), 0);
    };
    let category = match error.kind {
        FormatErrorKind::UnmatchedKeyParentheses => "unmatched-key-parentheses",
        FormatErrorKind::IncompleteFormat => "incomplete-format",
        FormatErrorKind::UnsupportedFormatCharacter => "unsupported-format-character",
        FormatErrorKind::WidthOrPrecisionTooLarge => "width-or-precision-too-large",
    };
    let character = error
        .character
        .map_or_else(String::new, |value| value.to_string());
    let index = i64::try_from(error.index).unwrap_or(i64::MAX);
    (category, character, index)
}

fn flag(value: bool) -> languages::Flag {
    if value {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

fn width_mode(value: FormatQuantityMode) -> &'static str {
    match value {
        FormatQuantityMode::None => "none",
        FormatQuantityMode::Fixed => "fixed",
        FormatQuantityMode::Star => "star",
        FormatQuantityMode::Dot => "dot",
    }
}

fn precision_mode(value: FormatQuantityMode) -> &'static str {
    width_mode(value)
}

ryo::reads! {
    FormatParseItem => languages::FormatParse, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        given index = prepare::<RuffLiteralCFormat>(tree, model, symbols);
        each (occurrences(&index)) as occurrence {
            require crate::domain::semantic_syntax::service::visibility(tree, model, occurrence.node, symbols) == Visibility::Visible;
            given parsed = result(&index, occurrence);
            given summary = summary_of(parsed, occurrence.unknown_input);
            given error_parts = error_parts(parsed);
            given dialect = intern(symbols, tree.source, "percent-string");
            given representation = intern(symbols, tree.source, "decoded-string");
            given outcome = intern(symbols, tree.source, summary.1);
            given error_category = intern(symbols, tree.source, error_parts.0);
            given error_character = intern(symbols, tree.source, &error_parts.1);
            given error_index = languages::Integer(error_parts.2);
            given positional_consumption = languages::Integer(i64::try_from(summary.0.positional_consumption).unwrap_or(i64::MAX));
            row { id: node(occurrence.node), dialect: value(dialect), representation: value(representation), outcome: value(outcome), error_category: value(error_category), error_character: value(error_character), error_index: value(error_index), positional_consumption: value(positional_consumption), mapping_present: value(flag(summary.0.mapping_present)), dynamic_width: value(flag(summary.0.dynamic_width)), dynamic_precision: value(flag(summary.0.dynamic_precision)), file: file, span: span(occurrence.node), }
        }
        when state(&index) {
            FormatParseState::Partial => { partial INCOMPLETE => languages::Missing::Any; }
            FormatParseState::Fresh => { partial INCOMPLETE => languages::Missing::Any; }
            FormatParseState::Complete => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FormatFieldItem, FormatParseItem};
    use crate::domain::relations::collections::fixtures::contract::check;
    use crate::fixtures::reading::reading;
    use ryo::languages::contract::Want;

    #[test]
    fn parse_rows_distinguish_zero_fields_malformed_and_dynamic_inputs() {
        check::<FormatParseItem>();
        let report = reading::<FormatParseItem>(concat!(
            "a = '100%%'\n",
            "b = '%*.*s'\n",
            "c = '%(name)s %(name)r'\n",
            "d = 'é%n'\n",
            "e = f'%s {value}'\n",
            "f = 'left' ' %s'\n",
            "g = f'100%%'\n",
        ));
        assert!(report.causes.is_empty(), "{:?}", report.causes);
        assert_eq!(report.rows.len(), 7);
        let outcomes = report
            .rows
            .iter()
            .map(|row| match row[3] {
                Want::Name(value) => value,
                _ => panic!("format outcome is a name"),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            outcomes,
            [
                "parsed",
                "parsed",
                "parsed",
                "malformed",
                "unknown-input",
                "parsed",
                "parsed",
            ]
        );
        assert_eq!(report.rows[0][7], Want::Number(0));
        assert_eq!(report.rows[1][7], Want::Number(3));
        assert_eq!(report.rows[1][9], Want::Flag(true));
        assert_eq!(report.rows[1][10], Want::Flag(true));
        assert_eq!(report.rows[2][8], Want::Flag(true));
        assert_eq!(report.rows[2][7], Want::Number(0));
        assert_eq!(
            report.rows[3][4],
            Want::Name("unsupported-format-character")
        );
        assert_eq!(
            report.rows[3][6],
            Want::Number(2),
            "Unicode index counts scalars"
        );
    }

    #[test]
    fn conversion_rows_keep_repeated_mapping_keys_and_order() {
        check::<FormatFieldItem>();
        let report = reading::<FormatFieldItem>("value = '%(key)s %*.*r %% %s'\n");
        assert!(report.causes.is_empty(), "{:?}", report.causes);
        assert_eq!(report.rows.len(), 3);
        assert_eq!(report.rows[0][1], Want::Number(0));
        assert_eq!(report.rows[0][2], Want::Name("mapping"));
        assert_eq!(report.rows[0][3], Want::Name("key"));
        assert_eq!(report.rows[1][1], Want::Number(1));
        assert_eq!(report.rows[1][2], Want::Name("positional"));
        assert_eq!(report.rows[1][5], Want::Name("star"));
        assert_eq!(report.rows[1][6], Want::Name("star"));
        assert_eq!(report.rows[2][1], Want::Number(2));
    }

    #[test]
    fn unknown_and_malformed_inputs_have_no_conversion_rows() {
        let dynamic = reading::<FormatFieldItem>("value = f'%s {value}'\n");
        assert!(dynamic.causes.is_empty(), "{:?}", dynamic.causes);
        assert!(dynamic.rows.is_empty());
        let malformed = reading::<FormatFieldItem>("value = '%n'\n");
        assert!(malformed.causes.is_empty(), "{:?}", malformed.causes);
        assert!(malformed.rows.is_empty());
    }

    #[test]
    fn template_strings_are_not_percent_string_inputs() {
        let report = reading::<FormatParseItem>("value = t'%s'\n");
        assert!(report.causes.is_empty(), "{:?}", report.causes);
        assert!(report.rows.is_empty());
    }
}

ryo::reads! {
    FormatFieldItem => languages::FormatField, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        given index = prepare::<RuffLiteralCFormat>(tree, model, symbols);
        each (occurrences(&index)) as occurrence {
            require crate::domain::semantic_syntax::service::visibility(tree, model, occurrence.node, symbols) == Visibility::Visible;
            given parsed = result(&index, occurrence);
            when parsed {
                Some(FormatParseResult::Parsed(parsed)) => {
                    each (parsed.fields) as field {
                        given argument_kind = intern(symbols, tree.source, match &field.argument { FormatArgument::Positional => "positional", FormatArgument::Mapping(_) => "mapping" });
                        given mapping_key = intern(symbols, tree.source, match &field.argument { FormatArgument::Positional => "", FormatArgument::Mapping(key) => key });
                        given conversion = intern(symbols, tree.source, &field.conversion.to_string());
                        given width = intern(symbols, tree.source, width_mode(field.width));
                        given precision = intern(symbols, tree.source, precision_mode(field.precision));
                        given ordinal = languages::Integer(i64::try_from(field.ordinal).unwrap_or(i64::MAX));
                        row { id: node(occurrence.node), ordinal: value(ordinal), argument_kind: value(argument_kind), mapping_key: value(mapping_key), conversion: value(conversion), width_mode: value(width), precision_mode: value(precision), file: file, span: span(occurrence.node), }
                    }
                }
                Some(FormatParseResult::Malformed(_)) => {}
                Some(FormatParseResult::Unavailable) => {}
                None => {}
            }
        }
        when state(&index) {
            FormatParseState::Partial => { partial INCOMPLETE => languages::Missing::Any; }
            FormatParseState::Fresh => { partial INCOMPLETE => languages::Missing::Any; }
            FormatParseState::Complete => {}
        }
    }
}
