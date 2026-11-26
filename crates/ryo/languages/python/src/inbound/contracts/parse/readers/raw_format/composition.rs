//! Static raw-token reader binding; ordinary decoded format demand remains independent.

use crate::domain::file_view::types::PythonWalk;
use crate::domain::format_syntax::service::flag_text;
use crate::domain::format_syntax::types::{FormatDialect, FormatInput};
use crate::domain::literal_facts::service::string_tokens;
use crate::domain::model::names::service::fixed_word;
use crate::domain::model::names::types::FixedWords;
use crate::domain::node::service::{keyed_id, semantic_node, node_cell, semantic_nodes_of_kind, span_at, word, written};
use crate::domain::node::types::{Numbered, SyntaxFile};
use crate::domain::raw_format::constants::INCOMPLETE;
use crate::domain::raw_format::service::body;
use crate::domain::raw_format::types::RawFormatToken;
use crate::domain::{format_syntax::types::{DynamicInput, FormatParseResult}, nodes::types::Node, syntax::{types::Kind}};
use crate::inbound::contracts::parse::readers::format::composition::{argument_kind, error_parts, mapping_key, property_flag, quantity_mode, summary_of};
use crate::inbound::contracts::parse::readers::format::constants::NO_CHARACTER;
use crate::inbound::contracts::parse::readers::format::constants::PERCENT_STRING;
use crate::outbound::format_syntax::adapters::ruff_literal_cformat::grammar::RuffLiteralCFormat;
use crate::outbound::format_syntax::interface::FormatSyntaxPort as _;
use ruff_python_ast::AnyNodeRef;
use ryo::languages;

/// Reads one parse outcome for each original raw string token.
pub(crate) struct RawFormatSegmentItem;
/// Reads fields parsed independently from each original raw string token.
pub(crate) struct RawFormatFieldItem;

/// The string tokens of the string at `node` into `tokens`, in the order they are written, each with the percent-format parse of
/// its body as it is written into `parses`: none for a token whose body cannot be told from its quotes. Nothing for a node that is
/// no string.
fn raw_segments(
    file: &SyntaxFile<'_>,
    node: AnyNodeRef<'_>,
    tokens: &mut Vec<languages::Position>,
    parses: &mut Vec<Option<FormatParseResult>>,
) {
    parses.clear();
    string_tokens(file, node, tokens);
    parses.extend(tokens.iter().map(|token| {
        let spelling = languages::node_table::decoded_of(file.table, *token).unwrap_or_else(|| written(file, *token));
        body(RawFormatToken(spelling.0))
            .map(|raw| RuffLiteralCFormat.parse(FormatInput(raw.0), FormatDialect::PercentString))
    }));
}

/// Whether every token of a string was parsed: a string with no token, or with a token whose body is not told or whose grammar
/// is unavailable, is read only in part.
fn segments_whole(parses: &[Option<FormatParseResult>]) -> languages::SyntaxCompleteness {
    let unavailable = |parsed: &Option<FormatParseResult>| {
        matches!(parsed, None | Some(FormatParseResult::Unavailable))
    };
    if parses.is_empty() || parses.iter().any(unavailable) {
        languages::SyntaxCompleteness::Incomplete
    } else {
        languages::SyntaxCompleteness::Complete
    }
}

/// Whether the file was read whole, by its parser and its lexer.
fn source_whole(file: &SyntaxFile<'_>) -> languages::SyntaxCompleteness {
    if file.completeness.syntax == languages::SyntaxCompleteness::Incomplete {
        languages::SyntaxCompleteness::Incomplete
    } else {
        file.completeness.lexical
    }
}

/// What the segments of one string are read through: the worker's fixed names, and the string's tokens with the parse of each.
#[derive(Debug)]
struct SegmentRead<'m> {
    /// The fixed names written so far.
    memo: &'m mut FixedWords,
    /// The string's tokens, in the order they are written.
    tokens: Vec<languages::Position>,
    /// The percent-format parse of each token's body.
    parses: Vec<Option<FormatParseResult>>,
}

/// Writes the rows of the string `node` at `id`, one for each of its tokens, and says whether every token of it was parsed.
/// Nothing, and whole, for a node that is no string.
fn segment_rows(
    syntax: &SyntaxFile<'_>,
    words: &mut languages::FileWords,
    read: &mut SegmentRead<'_>,
    Numbered { id, node }: Numbered<'_>,
    rows: &mut dyn languages::Rows<languages::RawFormatSegmentRow>,
) -> languages::SyntaxCompleteness {
    if !matches!(node, AnyNodeRef::ExprStringLiteral(_)) {
        return languages::SyntaxCompleteness::Complete;
    }
    raw_segments(syntax, node, &mut read.tokens, &mut read.parses);
    for (ordinal, (token, parsed)) in read.tokens.iter().zip(&read.parses).enumerate() {
        let summary = summary_of(parsed.as_ref(), DynamicInput::NotObserved);
        let errors = error_parts(parsed.as_ref());
        let mut buffer = [0_u8; 4];
        let character = match errors.character {
            Some(character) => &*character.0.encode_utf8(&mut buffer),
            None => NO_CHARACTER,
        };
        rows.write(languages::RawFormatSegmentRow {
            id: node_cell(syntax, id),
            segment: node_cell(syntax, *token),
            segment_ordinal: languages::Integer(i64::try_from(ordinal).unwrap_or(i64::MAX)),
            dialect: fixed_word(words, read.memo, languages::NameText(PERCENT_STRING)),
            outcome: fixed_word(words, read.memo, summary.outcome),
            error_category: fixed_word(words, read.memo, errors.category),
            error_character: word(words, languages::NameText(character)),
            error_index: errors.index,
            positional_consumption: languages::Integer(
                i64::try_from(summary.summary.positional_consumption.0).unwrap_or(i64::MAX),
            ),
            mapping_present: property_flag(summary.summary.mapping_present),
            dynamic_width: property_flag(summary.summary.dynamic_width),
            dynamic_precision: property_flag(summary.summary.dynamic_precision),
            file: syntax.file,
            span: span_at(syntax, *token),
        });
    }
    segments_whole(&read.parses)
}

// The engine fills `raw-format-segment` from the walked file: every row of the file, or the rows of one string, read from that
// string's own tokens. A fill of one string says the relation is read in part when the file was not read whole or that string's
// tokens were not all parsed: it reads no other string.
ryo::fills! {
    RawFormatSegmentItem => languages::RawFormatSegment, walk(PythonWalk) |file, words, rows| {
        let syntax = file.syntax;
        let mut whole = source_whole(&syntax);
        let mut read = SegmentRead {
            memo: &mut file.model.words.get_mut().fixed,
            tokens: Vec::new(),
            parses: Vec::new(),
        };
        for numbered in semantic_nodes_of_kind(&syntax, Kind::Node(Node::Constant)) {
            if segment_rows(&syntax, words, &mut read, numbered, rows) == languages::SyntaxCompleteness::Incomplete {
                whole = languages::SyntaxCompleteness::Incomplete;
            }
        }
        if whole == languages::SyntaxCompleteness::Incomplete {
            rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
        }
    }
    key(languages::Column::RawFormatSegment(languages::RawFormatSegmentColumn::Id)) |file, words, key, rows| {
        let syntax = file.syntax;
        let mut whole = source_whole(&syntax);
        let found = keyed_id(syntax.file, key).and_then(|id| semantic_node(&syntax, id).map(|node| Numbered { id, node }));
        if let Some(numbered) = found {
            let mut read = SegmentRead {
                memo: &mut file.model.words.get_mut().fixed,
                tokens: Vec::new(),
                parses: Vec::new(),
            };
            if segment_rows(&syntax, words, &mut read, numbered, rows) == languages::SyntaxCompleteness::Incomplete {
                whole = languages::SyntaxCompleteness::Incomplete;
            }
        }
        if whole == languages::SyntaxCompleteness::Incomplete {
            rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
        }
    }
}

// The engine fills `raw-format-field` from the walked file: every row of the file, once.
ryo::fills! {
    RawFormatFieldItem => languages::RawFormatField, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::RawFormatField as languages::CatalogRelation>::Row;
        let memo = &mut file.model.words.get_mut().fixed;
        let syntax = file.syntax;
        let source = source_whole(&syntax);
        let mut whole = source;
        let (mut tokens, mut parses) = (Vec::new(), Vec::new());
        for Numbered { id, node } in semantic_nodes_of_kind(&syntax, Kind::Node(Node::Constant)) {
            if !matches!(node, AnyNodeRef::ExprStringLiteral(_)) {
                continue;
            }
            raw_segments(&syntax, node, &mut tokens, &mut parses);
            if segments_whole(&parses) == languages::SyntaxCompleteness::Incomplete {
                whole = languages::SyntaxCompleteness::Incomplete;
                continue;
            }
            if source == languages::SyntaxCompleteness::Incomplete {
                continue;
            }
            let mut first_field = 0_usize;
            for (ordinal, (token, parsed)) in tokens.iter().zip(&parses).enumerate() {
                let Some(FormatParseResult::Parsed(parsed)) = parsed else {
                    continue;
                };
                for field in &parsed.fields {
                    let mut buffer = [0_u8; 4];
                    let conversion = &*field.conversion.0.encode_utf8(&mut buffer);
                    let among = first_field.saturating_add(field.ordinal.0);
                    rows.write(Row {
                        id: node_cell(&syntax, id),
                        segment: node_cell(&syntax, *token),
                        segment_ordinal: languages::Integer(i64::try_from(ordinal).unwrap_or(i64::MAX)),
                        ordinal: languages::Integer(i64::try_from(among).unwrap_or(i64::MAX)),
                        segment_field_ordinal: languages::Integer(
                            i64::try_from(field.ordinal.0).unwrap_or(i64::MAX),
                        ),
                        argument_kind: fixed_word(words, memo, argument_kind(&field.argument)),
                        mapping_key: word(words, mapping_key(&field.argument)),
                        conversion: word(words, languages::NameText(conversion)),
                        width_mode: fixed_word(words, memo, quantity_mode(field.width)),
                        precision_mode: fixed_word(words, memo, quantity_mode(field.precision)),
                        file: syntax.file,
                        span: span_at(&syntax, *token),
                        flags: fixed_word(words, memo, languages::NameText(flag_text(field.flags).0)),
                    });
                }
                first_field = first_field.saturating_add(parsed.fields.len());
            }
        }
        if whole == languages::SyntaxCompleteness::Incomplete {
            rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
        }
    }
}
