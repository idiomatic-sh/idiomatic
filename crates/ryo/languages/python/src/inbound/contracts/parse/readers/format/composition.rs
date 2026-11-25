//! Static binding from the Python composition to its percent-format grammar adapter.

use super::constants::{
    DECODED_PERCENT_STRING, DOT_QUANTITY, FIXED_QUANTITY, INCOMPLETE_FORMAT, MAPPING, NO_CHARACTER,
    NO_ERROR_CATEGORY, NO_KEY, NO_QUANTITY, PERCENT_MALFORMED, PERCENT_PARSED, PERCENT_STRING,
    PERCENT_UNAVAILABLE, PERCENT_UNKNOWN_INPUT, POSITIONAL, STAR_QUANTITY,
    UNMATCHED_KEY_PARENTHESES, UNSUPPORTED_FORMAT_CHARACTER, WIDTH_OR_PRECISION_TOO_LARGE,
};
use super::types::{ErrorParts, Summarized};
use crate::application::format_parsing::service::prepare;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::format_parsing::constants::INCOMPLETE;
use crate::domain::format_parsing::service::{occurrences, result, state};
use crate::domain::format_syntax::types::{
    DynamicInput, FormatArgument, FormatCharacter, FormatErrorKind, FormatParseResult,
    FormatParseState, FormatProperty, FormatQuantityMode, FormatSummary,
};
use crate::domain::format_syntax::types::{FormatDialect, FormatInput};
use crate::domain::literal_facts::service::string_value;
use crate::domain::literal_facts::types::{StringValue, TextBuffer};
use crate::domain::model::names::service::fixed_word;
use crate::domain::node::service::{node_cell, semantic_nodes_of_kind, span_at, word};
use crate::domain::node::types::{Numbered, SyntaxFile};
use crate::domain::{
    model::types::Declarations, nodes::types::Node, semantic_syntax::types::Visibility,
    syntax::service::kind_bits, syntax::types::Kind,
};
use crate::outbound::format_syntax::adapters::ruff_literal_cformat::grammar::RuffLiteralCFormat;
use crate::outbound::format_syntax::interface::FormatSyntaxPort as _;
use ryo::languages;

/// Reads whole-token percent-format parse summaries.
pub(crate) struct FormatParseItem;
/// Reads ordered conversion fields from whole-token percent-format strings.
pub(crate) struct FormatFieldItem;

/// Interns text in the Python composition's source arena.
pub(crate) fn intern(
    symbols: &mut languages::Symbols,
    source: languages::ByteSlice<'_>,
    value: languages::NameText<'_>,
) -> languages::Symbol {
    languages::intern_derived(symbols, source, value)
}

/// Intern one Unicode scalar from stack storage, or the empty diagnostic default.
pub(crate) fn intern_character(
    symbols: &mut languages::Symbols,
    source: languages::ByteSlice<'_>,
    character: Option<FormatCharacter>,
) -> languages::Symbol {
    let mut buffer = [0_u8; 4];
    let text = match character {
        Some(character) => character.0.encode_utf8(&mut buffer),
        None => NO_CHARACTER,
    };
    intern(symbols, source, languages::NameText(text))
}

/// Returns the parse summary and its stable outcome label.
pub(crate) fn summary_of(result: Option<&FormatParseResult>, unknown: DynamicInput) -> Summarized {
    let summarized = |summary, outcome| Summarized {
        summary,
        outcome: languages::NameText(outcome),
    };
    match result {
        Some(FormatParseResult::Parsed(parsed)) => summarized(parsed.summary, PERCENT_PARSED),
        Some(FormatParseResult::Malformed(_)) => {
            summarized(FormatSummary::default(), PERCENT_MALFORMED)
        }
        None if unknown == DynamicInput::Observed => {
            summarized(FormatSummary::default(), PERCENT_UNKNOWN_INPUT)
        }
        Some(FormatParseResult::Unavailable) | None => {
            summarized(FormatSummary::default(), PERCENT_UNAVAILABLE)
        }
    }
}

/// Returns the error category, character and scalar index, or empty defaults.
pub(crate) fn error_parts(result: Option<&FormatParseResult>) -> ErrorParts {
    let Some(FormatParseResult::Malformed(error)) = result else {
        return ErrorParts {
            category: languages::NameText(NO_ERROR_CATEGORY),
            character: None,
            index: languages::Integer(0),
        };
    };
    let category = match error.kind {
        FormatErrorKind::UnmatchedKeyParentheses => UNMATCHED_KEY_PARENTHESES,
        FormatErrorKind::IncompleteFormat => INCOMPLETE_FORMAT,
        FormatErrorKind::UnsupportedFormatCharacter => UNSUPPORTED_FORMAT_CHARACTER,
        FormatErrorKind::WidthOrPrecisionTooLarge => WIDTH_OR_PRECISION_TOO_LARGE,
    };
    ErrorParts {
        category: languages::NameText(category),
        character: error.character,
        index: languages::Integer(i64::try_from(error.index.0).unwrap_or(i64::MAX)),
    }
}

/// Converts a format property to the relation's typed flag.
pub(crate) fn property_flag(property: FormatProperty) -> languages::Flag {
    if bool::from(property) {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// Names how a field obtains its width or precision.
pub(crate) fn quantity_mode(value: FormatQuantityMode) -> languages::NameText<'static> {
    languages::NameText(match value {
        FormatQuantityMode::None => NO_QUANTITY,
        FormatQuantityMode::Fixed => FIXED_QUANTITY,
        FormatQuantityMode::Star => STAR_QUANTITY,
        FormatQuantityMode::Dot => DOT_QUANTITY,
    })
}

/// Names how a field selects its argument.
pub(crate) fn argument_kind(argument: &FormatArgument) -> languages::NameText<'static> {
    languages::NameText(match argument {
        FormatArgument::Positional => POSITIONAL,
        FormatArgument::Mapping(_) => MAPPING,
    })
}

/// Returns the mapping key a field selects, or none.
pub(crate) fn mapping_key(argument: &FormatArgument) -> languages::NameText<'_> {
    languages::NameText(match argument {
        FormatArgument::Positional => NO_KEY,
        FormatArgument::Mapping(key) => key.0.as_str(),
    })
}

ryo::reads! {
    FormatParseItem => languages::FormatParse, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits, expanded,
        given index = prepare(&RuffLiteralCFormat, tree, &model.format_parsing, symbols, crate::domain::semantic_syntax::service::positions(tree), |node, symbols| (crate::domain::semantic_syntax::service::visibility(tree, model, node, symbols) == Visibility::Visible).into());
        each (occurrences(&index)) as occurrence {
            require crate::domain::semantic_syntax::service::visibility(tree, model, occurrence.node, symbols) == Visibility::Visible;
            given parsed = result(&index, occurrence);
            given summary = summary_of(parsed, occurrence.unknown_input);
            given error_parts = error_parts(parsed);
            given dialect = intern(symbols, tree.source, languages::NameText(PERCENT_STRING));
            given representation = intern(symbols, tree.source, languages::NameText(DECODED_PERCENT_STRING));
            given outcome = intern(symbols, tree.source, summary.outcome);
            given error_category = intern(symbols, tree.source, error_parts.category);
            given error_character = intern_character(symbols, tree.source, error_parts.character);
            given error_index = error_parts.index;
            given positional_consumption = languages::Integer(i64::try_from(summary.summary.positional_consumption.0).unwrap_or(i64::MAX));
            row { id: node(occurrence.node), dialect: value(dialect), representation: value(representation), outcome: value(outcome), error_category: value(error_category), error_character: value(error_character), error_index: value(error_index), positional_consumption: value(positional_consumption), mapping_present: value(property_flag(summary.summary.mapping_present)), dynamic_width: value(property_flag(summary.summary.dynamic_width)), dynamic_precision: value(property_flag(summary.summary.dynamic_precision)), file: file, span: span(occurrence.node), }
        }
        when state(&index) {
            FormatParseState::Partial | FormatParseState::Fresh => { partial INCOMPLETE => languages::Missing::Any; }
            FormatParseState::Complete => {}
        }
    }
}

ryo::reads! {
    FormatFieldItem => languages::FormatField, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits, expanded,
        given index = prepare(&RuffLiteralCFormat, tree, &model.format_parsing, symbols, crate::domain::semantic_syntax::service::positions(tree), |node, symbols| (crate::domain::semantic_syntax::service::visibility(tree, model, node, symbols) == Visibility::Visible).into());
        each (occurrences(&index)) as occurrence {
            require crate::domain::semantic_syntax::service::visibility(tree, model, occurrence.node, symbols) == Visibility::Visible;
            when result(&index, occurrence) {
                Some(FormatParseResult::Parsed(parsed)) => {
                    each (parsed.fields.iter()) as field {
                        given argument_kind = intern(symbols, tree.source, argument_kind(&field.argument));
                        given mapping_key = intern(symbols, tree.source, mapping_key(&field.argument));
                        given conversion = intern_character(symbols, tree.source, Some(field.conversion));
                        given width = intern(symbols, tree.source, quantity_mode(field.width));
                        given precision = intern(symbols, tree.source, quantity_mode(field.precision));
                        given ordinal = languages::Integer(i64::try_from(field.ordinal.0).unwrap_or(i64::MAX));
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

/// Hands `each` every string occurrence of a walked file with its percent-format parse: a string and an f-string that stands on
/// its own are parsed as their decoded value; one that interpolates has an input no file states, and no parse.
fn parsed_strings(
    file: &SyntaxFile<'_>,
    joined: &mut TextBuffer,
    each: &mut dyn FnMut(languages::Position, Option<&FormatParseResult>, DynamicInput),
) {
    for kind in [Node::Constant, Node::JoinedStr] {
        for Numbered { id, node } in semantic_nodes_of_kind(file, Kind::Node(kind)) {
            match string_value(node, joined) {
                Some(StringValue::Known(text)) => {
                    let parsed =
                        RuffLiteralCFormat.parse(FormatInput(text.0), FormatDialect::PercentString);
                    each(id, Some(&parsed), DynamicInput::NotObserved);
                }
                Some(StringValue::Interpolated) => each(id, None, DynamicInput::Observed),
                None => {}
            }
        }
    }
}

/// Whether the file was read whole: a percent-format reading of a file the parser or the lexer refused a part of is partial.
fn read_whole(file: &SyntaxFile<'_>) -> languages::SyntaxCompleteness {
    if file.completeness.syntax == languages::SyntaxCompleteness::Incomplete {
        languages::SyntaxCompleteness::Incomplete
    } else {
        file.completeness.lexical
    }
}

// The engine fills `format-parse` from the walked file: every row of the file, once.
ryo::fills! {
    FormatParseItem => languages::FormatParse, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::FormatParse as languages::CatalogRelation>::Row;
        let memo = &mut file.model.words.get_mut().fixed;
        let syntax = file.syntax;
        let mut whole = read_whole(&syntax);
        parsed_strings(&syntax, &mut file.buffers.text, &mut |id, parsed, unknown| {
            let summary = summary_of(parsed, unknown);
            let error = error_parts(parsed);
            let mut buffer = [0_u8; 4];
            let character = match error.character {
                Some(character) => &*character.0.encode_utf8(&mut buffer),
                None => NO_CHARACTER,
            };
            if matches!(parsed, Some(FormatParseResult::Unavailable)) {
                whole = languages::SyntaxCompleteness::Incomplete;
            }
            rows.write(Row {
                id: node_cell(&syntax, id),
                dialect: fixed_word(words, memo, languages::NameText(PERCENT_STRING)),
                representation: fixed_word(words, memo, languages::NameText(DECODED_PERCENT_STRING)),
                outcome: fixed_word(words, memo, summary.outcome),
                error_category: fixed_word(words, memo, error.category),
                error_character: word(words, languages::NameText(character)),
                error_index: error.index,
                positional_consumption: languages::Integer(
                    i64::try_from(summary.summary.positional_consumption.0).unwrap_or(i64::MAX),
                ),
                mapping_present: property_flag(summary.summary.mapping_present),
                dynamic_width: property_flag(summary.summary.dynamic_width),
                dynamic_precision: property_flag(summary.summary.dynamic_precision),
                file: syntax.file,
                span: span_at(&syntax, id),
            });
        });
        if whole == languages::SyntaxCompleteness::Incomplete {
            rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
        }
    }
}

// The engine fills `format-field` from the walked file: every row of the file, once.
ryo::fills! {
    FormatFieldItem => languages::FormatField, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::FormatField as languages::CatalogRelation>::Row;
        let memo = &mut file.model.words.get_mut().fixed;
        let syntax = file.syntax;
        let mut whole = read_whole(&syntax);
        parsed_strings(&syntax, &mut file.buffers.text, &mut |id, parsed, _unknown| {
            if matches!(parsed, Some(FormatParseResult::Unavailable)) {
                whole = languages::SyntaxCompleteness::Incomplete;
            }
            let Some(FormatParseResult::Parsed(parsed)) = parsed else {
                return;
            };
            for field in &parsed.fields {
                let mut buffer = [0_u8; 4];
                let conversion = &*field.conversion.0.encode_utf8(&mut buffer);
                rows.write(Row {
                    id: node_cell(&syntax, id),
                    ordinal: languages::Integer(i64::try_from(field.ordinal.0).unwrap_or(i64::MAX)),
                    argument_kind: fixed_word(words, memo, argument_kind(&field.argument)),
                    mapping_key: word(words, mapping_key(&field.argument)),
                    conversion: word(words, languages::NameText(conversion)),
                    width_mode: fixed_word(words, memo, quantity_mode(field.width)),
                    precision_mode: fixed_word(words, memo, quantity_mode(field.precision)),
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
