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

/// Reads whole-token percent-format parse summaries.
pub(super) struct FormatParseItem;
/// Reads ordered conversion fields from whole-token percent-format strings.
pub(super) struct FormatFieldItem;

/// Interns text in the Python composition's source arena.
pub(super) fn intern(
    symbols: &mut languages::Symbols,
    source: languages::ByteSlice<'_>,
    value: &str,
) -> languages::Symbol {
    languages::intern_derived(symbols, source, languages::NameText(value))
}

/// Intern one Unicode scalar from stack storage, or the empty diagnostic default.
pub(super) fn intern_character(
    symbols: &mut languages::Symbols,
    source: languages::ByteSlice<'_>,
    character: Option<char>,
) -> languages::Symbol {
    let mut buffer = [0_u8; 4];
    let text = match character {
        Some(character) => character.encode_utf8(&mut buffer),
        None => "",
    };
    intern(symbols, source, text)
}

/// Returns the parse summary and its stable outcome label.
pub(super) fn summary_of(
    result: Option<&FormatParseResult>,
    unknown: bool,
) -> (FormatSummary, &'static str) {
    match result {
        Some(FormatParseResult::Parsed(parsed)) => (parsed.summary, "parsed"),
        Some(FormatParseResult::Malformed(_)) => (FormatSummary::default(), "malformed"),
        None if unknown => (FormatSummary::default(), "unknown-input"),
        Some(FormatParseResult::Unavailable) | None => (FormatSummary::default(), "unavailable"),
    }
}

/// Returns the error category, character and scalar index, or empty defaults.
pub(super) fn error_parts(result: Option<&FormatParseResult>) -> (&'static str, Option<char>, i64) {
    let Some(FormatParseResult::Malformed(error)) = result else {
        return ("none", None, 0);
    };
    let category = match error.kind {
        FormatErrorKind::UnmatchedKeyParentheses => "unmatched-key-parentheses",
        FormatErrorKind::IncompleteFormat => "incomplete-format",
        FormatErrorKind::UnsupportedFormatCharacter => "unsupported-format-character",
        FormatErrorKind::WidthOrPrecisionTooLarge => "width-or-precision-too-large",
    };
    let character = error.character;
    let index = i64::try_from(error.index).unwrap_or(i64::MAX);
    (category, character, index)
}

/// Converts a boolean to the relation's typed flag.
pub(super) fn flag(value: bool) -> languages::Flag {
    if value {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// Names how a field obtains its width or precision.
pub(super) fn width_mode(value: FormatQuantityMode) -> &'static str {
    match value {
        FormatQuantityMode::None => "none",
        FormatQuantityMode::Fixed => "fixed",
        FormatQuantityMode::Star => "star",
        FormatQuantityMode::Dot => "dot",
    }
}

/// Names the precision mode using the same vocabulary as width.
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
            given error_character = intern_character(symbols, tree.source, error_parts.1);
            given error_index = languages::Integer(error_parts.2);
            given positional_consumption = languages::Integer(i64::try_from(summary.0.positional_consumption).unwrap_or(i64::MAX));
            row { id: node(occurrence.node), dialect: value(dialect), representation: value(representation), outcome: value(outcome), error_category: value(error_category), error_character: value(error_character), error_index: value(error_index), positional_consumption: value(positional_consumption), mapping_present: value(flag(summary.0.mapping_present)), dynamic_width: value(flag(summary.0.dynamic_width)), dynamic_precision: value(flag(summary.0.dynamic_precision)), file: file, span: span(occurrence.node), }
        }
        when state(&index) {
            FormatParseState::Partial | FormatParseState::Fresh => { partial INCOMPLETE => languages::Missing::Any; }
            FormatParseState::Complete => {}
        }
    }
}

ryo::reads! {
    FormatFieldItem => languages::FormatField, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        given index = prepare::<RuffLiteralCFormat>(tree, model, symbols);
        each (occurrences(&index)) as occurrence {
            require crate::domain::semantic_syntax::service::visibility(tree, model, occurrence.node, symbols) == Visibility::Visible;
            when result(&index, occurrence) {
                Some(FormatParseResult::Parsed(parsed)) => {
                    each (parsed.fields.iter()) as field {
                        given argument_kind = intern(symbols, tree.source, match &field.argument { FormatArgument::Positional => "positional", FormatArgument::Mapping(_) => "mapping" });
                        given mapping_key = intern(symbols, tree.source, match &field.argument { FormatArgument::Positional => "", FormatArgument::Mapping(key) => key });
                        given conversion = intern_character(symbols, tree.source, Some(field.conversion));
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
            FormatParseState::Partial | FormatParseState::Fresh => { partial INCOMPLETE => languages::Missing::Any; }
            FormatParseState::Complete => {}
        }
    }
}

#[cfg(test)]
#[path = "format_readers_test.rs"]
mod tests;
