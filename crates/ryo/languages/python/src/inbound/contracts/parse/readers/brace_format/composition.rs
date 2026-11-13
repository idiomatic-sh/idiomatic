//! Static binding from the Python composition to the independent brace-format parser.
use super::constants::{
    AUTOMATIC, BRACE_MALFORMED, BRACE_PARSED, BRACE_UNAVAILABLE, DECODED_STRING, EMPTY_ATTRIBUTE,
    FORMAT_SPEC, INDEX, INVALID_CHARACTER_AFTER_RIGHT_BRACKET, KEYWORD, LITERAL,
    MISSING_RIGHT_BRACKET, MISSING_START_BRACKET, NO_ERROR, NO_NAME, NO_PARENT,
    PLACEHOLDER_RECURSION_EXCEEDED, UNKNOWN_CONVERSION, UNKNOWN_INPUT, UNMATCHED_BRACKET,
};
use super::types::{FieldCounts, OutcomeLabels};
use crate::application::brace_format::service::prepare;
use crate::domain::brace_format::constants::INCOMPLETE;
use crate::domain::brace_format::service::{occurrences, result, state};
use crate::domain::brace_format::types::{
    BraceFieldContext, BraceFormatError, BraceFormatResult, BraceFormatState, BraceSelector,
    SelectorRange,
};
use crate::domain::format_syntax::types::DynamicInput;
use crate::domain::{
    model::types::Declarations, nodes::types::Node, semantic_syntax::types::Visibility,
    syntax::service::kind_bits, syntax::types::Kind,
};
use crate::outbound::brace_format::adapters::ruff_literal_braceformat::grammar::RuffLiteralBraceFormat;
use ryo::languages;

/// One outcome row per source string occurrence.
pub(crate) struct BraceFormatParseItem;
/// Ordered selector rows for parsed fields.
pub(crate) struct BraceFormatFieldItem;

/// Interns text in the Python composition's source arena.
fn intern(
    symbols: &mut languages::Symbols,
    source: languages::ByteSlice<'_>,
    value: languages::NameText<'_>,
) -> languages::Symbol {
    languages::intern_derived(symbols, source, value)
}

/// Converts a selector range to the relation's typed flag.
fn range_flag(range: SelectorRange) -> languages::Flag {
    if bool::from(range) {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// Returns stable outcome and error labels for one source string.
fn outcome(result: Option<&BraceFormatResult>, unknown: DynamicInput) -> OutcomeLabels {
    let labels = |outcome, error| OutcomeLabels {
        outcome: languages::NameText(outcome),
        error,
    };
    match result {
        Some(BraceFormatResult::Parsed(_)) => labels(BRACE_PARSED, languages::NameText(NO_ERROR)),
        Some(BraceFormatResult::Malformed(error)) => labels(BRACE_MALFORMED, error_name(*error)),
        None if unknown == DynamicInput::Observed => {
            labels(UNKNOWN_INPUT, languages::NameText(NO_ERROR))
        }
        Some(BraceFormatResult::Unavailable) | None => {
            labels(BRACE_UNAVAILABLE, languages::NameText(NO_ERROR))
        }
    }
}

/// Names a malformed brace-format error for relation rows.
fn error_name(error: BraceFormatError) -> languages::NameText<'static> {
    languages::NameText(match error {
        BraceFormatError::UnmatchedBracket => UNMATCHED_BRACKET,
        BraceFormatError::MissingStartBracket => MISSING_START_BRACKET,
        BraceFormatError::PlaceholderRecursionExceeded => PLACEHOLDER_RECURSION_EXCEEDED,
        BraceFormatError::UnknownConversion => UNKNOWN_CONVERSION,
        BraceFormatError::EmptyAttribute => EMPTY_ATTRIBUTE,
        BraceFormatError::MissingRightBracket => MISSING_RIGHT_BRACKET,
        BraceFormatError::InvalidCharacterAfterRightBracket => {
            INVALID_CHARACTER_AFTER_RIGHT_BRACKET
        }
    })
}

/// Returns automatic, indexed, keyword and nested-field counts.
fn counts(result: Option<&BraceFormatResult>) -> FieldCounts {
    let Some(BraceFormatResult::Parsed(parsed)) = result else {
        return FieldCounts {
            automatic: languages::Integer(0),
            indexed: languages::Integer(0),
            keyword: languages::Integer(0),
            nested: languages::Integer(0),
        };
    };
    FieldCounts {
        automatic: languages::Integer(i64::try_from(parsed.automatic_count.0).unwrap_or(i64::MAX)),
        indexed: languages::Integer(i64::try_from(parsed.indexed_count.0).unwrap_or(i64::MAX)),
        keyword: languages::Integer(i64::try_from(parsed.keyword_count.0).unwrap_or(i64::MAX)),
        nested: languages::Integer(i64::try_from(parsed.nested_field_count.0).unwrap_or(i64::MAX)),
    }
}

/// Names where a field is written.
fn context_name(context: &BraceFieldContext) -> languages::NameText<'static> {
    languages::NameText(match context {
        BraceFieldContext::Literal => LITERAL,
        BraceFieldContext::FormatSpec { .. } => FORMAT_SPEC,
    })
}

/// Returns the ordinal of the field a nested field belongs to, or none.
fn parent_ordinal(context: &BraceFieldContext) -> languages::Integer {
    match context {
        BraceFieldContext::Literal => languages::Integer(NO_PARENT),
        BraceFieldContext::FormatSpec { parent_ordinal } => {
            languages::Integer(i64::try_from(parent_ordinal.0).unwrap_or(i64::MAX))
        }
    }
}

/// Names the kind of a field's selector.
fn selector_kind(selector: &BraceSelector) -> languages::NameText<'static> {
    languages::NameText(match selector {
        BraceSelector::Automatic { .. } => AUTOMATIC,
        BraceSelector::Index { .. } => INDEX,
        BraceSelector::Keyword(_) => KEYWORD,
    })
}

/// Returns the index a selector gives, or zero for a keyword.
fn selector_index(selector: &BraceSelector) -> languages::Integer {
    match selector {
        BraceSelector::Automatic { index, .. } | BraceSelector::Index { index, .. } => {
            languages::Integer(i64::try_from(index.0).unwrap_or(0))
        }
        BraceSelector::Keyword(_) => languages::Integer(0),
    }
}

/// Returns whether a selector's index overflows the relation's integer.
fn selector_overflow(selector: &BraceSelector) -> languages::Flag {
    match selector {
        BraceSelector::Automatic { overflow, .. } | BraceSelector::Index { overflow, .. } => {
            range_flag(*overflow)
        }
        BraceSelector::Keyword(_) => range_flag(SelectorRange::Representable),
    }
}

/// Returns the name a selector writes, or none.
fn selector_name(selector: &BraceSelector) -> languages::NameText<'_> {
    languages::NameText(match selector {
        BraceSelector::Automatic { .. } => NO_NAME,
        BraceSelector::Index { text, .. } => text.0.as_str(),
        BraceSelector::Keyword(name) => name.0.as_str(),
    })
}

ryo::reads! {
    BraceFormatParseItem => languages::BraceFormatParse, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits, expanded,
        given index = prepare(&RuffLiteralBraceFormat, tree, &model.brace_format, symbols, crate::domain::semantic_syntax::service::positions(tree), |node, symbols| (crate::domain::semantic_syntax::service::visibility(tree, model, node, symbols) == Visibility::Visible).into());
        each (occurrences(&index)) as occurrence {
            require crate::domain::semantic_syntax::service::visibility(tree, model, occurrence.node, symbols) == Visibility::Visible;
            given result = result(&index, occurrence);
            given state = outcome(result, occurrence.unknown_input);
            given counts = counts(result);
            given representation = intern(symbols, tree.source, languages::NameText(DECODED_STRING));
            given outcome = intern(symbols, tree.source, state.outcome);
            given error = intern(symbols, tree.source, state.error);
            given automatic_count = counts.automatic;
            given indexed_count = counts.indexed;
            given keyword_count = counts.keyword;
            given nested_field_count = counts.nested;
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
        kinds: [Kind::Node(Node::Module)] as kind_bits, expanded,
        given index = prepare(&RuffLiteralBraceFormat, tree, &model.brace_format, symbols, crate::domain::semantic_syntax::service::positions(tree), |node, symbols| (crate::domain::semantic_syntax::service::visibility(tree, model, node, symbols) == Visibility::Visible).into());
        each (occurrences(&index)) as occurrence {
            require crate::domain::semantic_syntax::service::visibility(tree, model, occurrence.node, symbols) == Visibility::Visible;
            when result(&index, occurrence) {
                Some(BraceFormatResult::Parsed(parsed)) => {
                    each (parsed.fields.iter()) as field {
                        given context = intern(symbols, tree.source, context_name(&field.context));
                        given parent_ordinal = parent_ordinal(&field.context);
                        given selector_kind = intern(symbols, tree.source, selector_kind(&field.selector));
                        given selector_index = selector_index(&field.selector);
                        given index_overflow = selector_overflow(&field.selector);
                        given selector_name = intern(symbols, tree.source, selector_name(&field.selector));
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
