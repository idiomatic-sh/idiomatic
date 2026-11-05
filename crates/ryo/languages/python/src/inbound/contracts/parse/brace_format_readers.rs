//! Static binding from the Python composition to the independent brace-format parser.
use crate::domain::brace_format::constants::INCOMPLETE;
use crate::domain::brace_format::service::{occurrences, prepare, result, state};
use crate::domain::brace_format::types::{
    BraceFieldContext, BraceFormatError, BraceFormatResult, BraceFormatState, BraceSelector,
};
use crate::domain::{
    model::types::Declarations, nodes::types::Node, semantic_syntax::types::Visibility,
    syntax::service::kind_bits, syntax::types::Kind,
};
use crate::outbound::format_syntax::adapters::ruff_literal_braceformat::RuffLiteralBraceFormat;
use ryo::languages;

/// One outcome row per source string occurrence.
pub(super) struct BraceFormatParseItem;
/// Ordered selector rows for parsed fields.
pub(super) struct BraceFormatFieldItem;

/// Interns text in the Python composition's source arena.
fn intern(
    symbols: &mut languages::Symbols,
    source: languages::ByteSlice<'_>,
    value: &str,
) -> languages::Symbol {
    languages::intern_derived(symbols, source, languages::NameText(value))
}

/// Converts a boolean to the relation's typed flag.
fn flag(value: bool) -> languages::Flag {
    if value {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// Returns stable outcome and error labels for one source string.
fn outcome(result: Option<&BraceFormatResult>, unknown: bool) -> (&'static str, &'static str) {
    match result {
        Some(BraceFormatResult::Parsed(_)) => ("parsed", "none"),
        Some(BraceFormatResult::Malformed(error)) => ("malformed", error_name(*error)),
        None if unknown => ("unknown-input", "none"),
        Some(BraceFormatResult::Unavailable) | None => ("unavailable", "none"),
    }
}

/// Names a malformed brace-format error for relation rows.
fn error_name(error: BraceFormatError) -> &'static str {
    match error {
        BraceFormatError::UnmatchedBracket => "unmatched-bracket",
        BraceFormatError::MissingStartBracket => "missing-start-bracket",
        BraceFormatError::PlaceholderRecursionExceeded => "placeholder-recursion-exceeded",
        BraceFormatError::UnknownConversion => "unknown-conversion",
        BraceFormatError::EmptyAttribute => "empty-attribute",
        BraceFormatError::MissingRightBracket => "missing-right-bracket",
        BraceFormatError::InvalidCharacterAfterRightBracket => {
            "invalid-character-after-right-bracket"
        }
    }
}

/// Returns automatic, indexed, keyword and nested-field counts.
fn counts(result: Option<&BraceFormatResult>) -> [i64; 4] {
    let Some(BraceFormatResult::Parsed(parsed)) = result else {
        return [0; 4];
    };
    [
        parsed.automatic_count,
        parsed.indexed_count,
        parsed.keyword_count,
        parsed.nested_field_count,
    ]
    .map(|value| i64::try_from(value.0).unwrap_or(i64::MAX))
}

ryo::reads! {
    BraceFormatParseItem => languages::BraceFormatParse, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        given index = prepare::<RuffLiteralBraceFormat>(tree, model, symbols);
        each (occurrences(&index)) as occurrence {
            require crate::domain::semantic_syntax::service::visibility(tree, model, occurrence.node, symbols) == Visibility::Visible;
            given result = result(&index, occurrence);
            given state = outcome(result, occurrence.unknown_input.into());
            given counts = counts(result);
            given representation = intern(symbols, tree.source, "decoded-string");
            given outcome = intern(symbols, tree.source, state.0);
            given error = intern(symbols, tree.source, state.1);
            given automatic_count = languages::Integer(counts[0]);
            given indexed_count = languages::Integer(counts[1]);
            given keyword_count = languages::Integer(counts[2]);
            given nested_field_count = languages::Integer(counts[3]);
            row { id: node(occurrence.node), representation: value(representation), outcome: value(outcome), error_category: value(error), automatic_count: value(automatic_count), indexed_count: value(indexed_count), keyword_count: value(keyword_count), nested_field_count: value(nested_field_count), file: file, span: span(occurrence.node), }
        }
        when state(&index) {
            BraceFormatState::Fresh | BraceFormatState::Partial => { partial INCOMPLETE => languages::Missing::Any; }
            BraceFormatState::Complete => {}
        }
    }
}

ryo::reads! {
    BraceFormatFieldItem => languages::BraceFormatField, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        given index = prepare::<RuffLiteralBraceFormat>(tree, model, symbols);
        each (occurrences(&index)) as occurrence {
            require crate::domain::semantic_syntax::service::visibility(tree, model, occurrence.node, symbols) == Visibility::Visible;
            when result(&index, occurrence) {
                Some(BraceFormatResult::Parsed(parsed)) => {
                    each (parsed.fields.iter()) as field {
                        given context = intern(symbols, tree.source, match field.context { BraceFieldContext::Literal => "literal", BraceFieldContext::FormatSpec { .. } => "format-spec" });
                        given parent = match field.context { BraceFieldContext::Literal => -1_i64, BraceFieldContext::FormatSpec { parent_ordinal } => i64::try_from(parent_ordinal.0).unwrap_or(i64::MAX) };
                        given parent_ordinal = languages::Integer(parent);
                        given selector_kind = intern(symbols, tree.source, match &field.selector { BraceSelector::Automatic { .. } => "automatic", BraceSelector::Index { .. } => "index", BraceSelector::Keyword(_) => "keyword" });
                        given selector_index = languages::Integer(match &field.selector { BraceSelector::Automatic { index, .. } | BraceSelector::Index { index, .. } => i64::try_from(index.0).unwrap_or(0), BraceSelector::Keyword(_) => 0 });
                        given overflow = match &field.selector { BraceSelector::Automatic { overflow, .. } | BraceSelector::Index { overflow, .. } => bool::from(*overflow), BraceSelector::Keyword(_) => false };
                        given index_overflow = flag(overflow);
                        given selector_name_text = match &field.selector { BraceSelector::Automatic { .. } => "", BraceSelector::Index { text, .. } => text.0.as_str(), BraceSelector::Keyword(name) => name.0.as_str() };
                        given selector_name = intern(symbols, tree.source, selector_name_text);
                        given ordinal = languages::Integer(i64::try_from(field.ordinal.0).unwrap_or(i64::MAX));
                        row { id: node(occurrence.node), ordinal: value(ordinal), context: value(context), parent_ordinal: value(parent_ordinal), selector_kind: value(selector_kind), selector_index: value(selector_index), index_overflow: value(index_overflow), selector_name: value(selector_name), file: file, span: span(occurrence.node), }
                    }
                }
                Some(BraceFormatResult::Malformed(_)) => {}
                Some(BraceFormatResult::Unavailable) => {}
                None => {}
            }
        }
        when state(&index) {
            BraceFormatState::Fresh | BraceFormatState::Partial => { partial INCOMPLETE => languages::Missing::Any; }
            BraceFormatState::Complete => {}
        }
    }
}

#[cfg(test)]
#[path = "brace_format_readers_test.rs"]
mod tests;
