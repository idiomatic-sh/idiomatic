//! Static binding from the Python composition to the independent brace-format parser.
use super::constants::{AUTOMATIC, BRACE_MALFORMED, BRACE_PARSED, BRACE_UNAVAILABLE, DECODED_STRING, EMPTY_ATTRIBUTE, FORMAT_SPEC, INDEX, INVALID_CHARACTER_AFTER_RIGHT_BRACKET, KEYWORD, LITERAL, MISSING_RIGHT_BRACKET, MISSING_START_BRACKET, NO_ERROR, NO_NAME, NO_PARENT, PLACEHOLDER_RECURSION_EXCEEDED, UNKNOWN_CONVERSION, UNKNOWN_INPUT, UNMATCHED_BRACKET};
use super::types::{FieldCounts, OutcomeLabels};

use crate::domain::brace_format::constants::INCOMPLETE;

use crate::domain::brace_format::types::{BraceFieldContext, BraceFormatError, BraceFormatResult, BraceSelector, SelectorRange};
use crate::domain::file_view::types::PythonWalk;
use crate::domain::format_syntax::types::DynamicInput;
use crate::domain::format_syntax::types::FormatInput;
use crate::domain::literal_facts::service::{format_spec_value, string_value};
use crate::domain::literal_facts::types::{StringValue, TextBuffer};
use crate::domain::model::names::service::fixed_word;
use crate::domain::node::service::{node_cell, semantic_nodes_of_kind, span_at, word};
use crate::domain::node::types::{Numbered, SyntaxFile};
use crate::domain::{nodes::types::Node, syntax::types::Kind};
use crate::outbound::brace_format::adapters::ruff_literal_braceformat::grammar::RuffLiteralBraceFormat;
use crate::outbound::brace_format::interface::BraceFormatSyntaxPort as _;
use ryo::languages;

/// One outcome row per source string occurrence.
pub(crate) struct BraceFormatParseItem;
/// Ordered selector rows for parsed fields.
pub(crate) struct BraceFormatFieldItem;

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
        None => {
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
pub(crate) fn parent_ordinal(context: &BraceFieldContext) -> languages::Integer {
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

/// Hands `each` every string occurrence of a walked file with its brace-format parse: a string, an f-string and the format spec
/// of an interpolation are parsed as their decoded text; one that interpolates has an input no file states, and no parse.
pub(crate) fn parsed_strings(
    file: &SyntaxFile<'_>,
    joined: &mut TextBuffer,
    each: &mut dyn FnMut(languages::Position, Option<&BraceFormatResult>, DynamicInput),
) {
    for kind in [Node::Constant, Node::JoinedStr] {
        for Numbered { id, node } in semantic_nodes_of_kind(file, Kind::Node(kind)) {
            let value = match format_spec_value(node, joined) {
                Some(spec) => Some(spec),
                None => string_value(node, joined),
            };
            match value {
                Some(StringValue::Known(text)) => {
                    let parsed = RuffLiteralBraceFormat.parse(FormatInput(text.0));
                    each(id, Some(&parsed), DynamicInput::NotObserved);
                }
                Some(StringValue::Interpolated) => each(id, None, DynamicInput::Observed),
                None => {}
            }
        }
    }
}

// The engine fills `brace-format-parse` from the walked file: every row of the file, once.
ryo::fills! {
    BraceFormatParseItem => languages::BraceFormatParse, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::BraceFormatParse as languages::CatalogRelation>::Row;
        let memo = &mut file.model.words.get_mut().fixed;
        let syntax = file.syntax;
        let mut whole = languages::SyntaxCompleteness::Complete;
        parsed_strings(&syntax, &mut file.buffers.text, &mut |id, parsed, unknown| {
            let labels = outcome(parsed, unknown);
            let found = counts(parsed);
            rows.write(Row {
                id: node_cell(&syntax, id),
                representation: fixed_word(words, memo, languages::NameText(DECODED_STRING)),
                outcome: fixed_word(words, memo, labels.outcome),
                error_category: fixed_word(words, memo, labels.error),
                automatic_count: found.automatic,
                indexed_count: found.indexed,
                keyword_count: found.keyword,
                nested_field_count: found.nested,
                file: syntax.file,
                span: span_at(&syntax, id),
            });
        });
        if whole == languages::SyntaxCompleteness::Incomplete {
            rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
        }
    }
}

// The engine fills `brace-format-field` from the walked file: every row of the file, once.
ryo::fills! {
    BraceFormatFieldItem => languages::BraceFormatField, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::BraceFormatField as languages::CatalogRelation>::Row;
        let memo = &mut file.model.words.get_mut().fixed;
        let syntax = file.syntax;
        let mut whole = languages::SyntaxCompleteness::Complete;
        parsed_strings(&syntax, &mut file.buffers.text, &mut |id, parsed, _unknown| {
            let Some(BraceFormatResult::Parsed(parsed)) = parsed else {
                return;
            };
            for field in &parsed.fields {
                rows.write(Row {
                    id: node_cell(&syntax, id),
                    ordinal: languages::Integer(i64::try_from(field.ordinal.0).unwrap_or(i64::MAX)),
                    context: fixed_word(words, memo, context_name(&field.context)),
                    parent_ordinal: parent_ordinal(&field.context),
                    selector_kind: fixed_word(words, memo, selector_kind(&field.selector)),
                    selector_index: selector_index(&field.selector),
                    index_overflow: selector_overflow(&field.selector),
                    selector_name: word(words, selector_name(&field.selector)),
                    file: syntax.file,
                    span: span_at(&syntax, id),
                });
            }
        });
        if whole == languages::SyntaxCompleteness::Incomplete {
            rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
        }
    }
}
