//! Static raw-token reader binding; ordinary decoded format demand remains independent.
use crate::application::raw_format::service::prepare;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::format_syntax::service::flag_text;
use crate::domain::format_syntax::types::{FormatDialect, FormatInput};
use crate::domain::node::service::{
    fixed, id_of, node_cell, nodes_of_kind, span_at, word, written,
};
use crate::domain::node::types::{Numbered, SyntaxFile};
use crate::domain::raw_format::constants::INCOMPLETE;
use crate::domain::raw_format::service::body;
use crate::domain::raw_format::types::RawFormatToken;
use crate::domain::syntax::service::kind_of;
use crate::domain::tokens::types::Token;
use crate::domain::{
    format_syntax::types::{DynamicInput, FormatParseResult, FormatParseState},
    model::types::Declarations,
    nodes::types::Node,
    raw_format::service::{result, segments, state},
    syntax::{service::kind_bits, types::Kind},
};
use crate::inbound::contracts::parse::readers::format::composition::{
    argument_kind, error_parts, intern, intern_character, mapping_key, property_flag,
    quantity_mode, summary_of,
};
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

ryo::reads! {
    RawFormatSegmentItem => languages::RawFormatSegment, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits, expanded,
        given index = prepare(&RuffLiteralCFormat, tree, &model.raw_format, symbols, crate::domain::semantic_syntax::service::positions(tree), |node, symbols| (crate::domain::semantic_syntax::service::visibility(tree, model, node, symbols) == crate::domain::semantic_syntax::types::Visibility::Visible).into());
        each (index.occurrences.iter()) as occurrence {
            each (segments(&index, occurrence)) as segment {
                given parsed = result(&index, segment);
                given summary = summary_of(parsed, DynamicInput::NotObserved);
                given errors = error_parts(parsed);
                given dialect = intern(symbols, tree.source, languages::NameText(PERCENT_STRING));
                given outcome = intern(symbols, tree.source, summary.outcome);
                given error_category = intern(symbols, tree.source, errors.category);
                given error_character = intern_character(symbols, tree.source, errors.character);
                given segment_ordinal = languages::Integer(i64::try_from(segment.ordinal.0).unwrap_or(i64::MAX));
                given positional_consumption = languages::Integer(i64::try_from(summary.summary.positional_consumption.0).unwrap_or(i64::MAX));
                row { id: node(occurrence.node), segment: node(segment.token), segment_ordinal: value(segment_ordinal), dialect: value(dialect), outcome: value(outcome), error_category: value(error_category), error_character: value(error_character), error_index: value(errors.index), positional_consumption: value(positional_consumption), mapping_present: value(property_flag(summary.summary.mapping_present)), dynamic_width: value(property_flag(summary.summary.dynamic_width)), dynamic_precision: value(property_flag(summary.summary.dynamic_precision)), file: file, span: span(segment.token), }
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
        kinds: [Kind::Node(Node::Module)] as kind_bits, expanded,
        given index = prepare(&RuffLiteralCFormat, tree, &model.raw_format, symbols, crate::domain::semantic_syntax::service::positions(tree), |node, symbols| (crate::domain::semantic_syntax::service::visibility(tree, model, node, symbols) == crate::domain::semantic_syntax::types::Visibility::Visible).into());
        each (index.occurrences.iter()) as occurrence {
            require bool::from(occurrence.complete);
            each (segments(&index, occurrence)) as segment {
                when result(&index, segment) {
                    Some(FormatParseResult::Parsed(parsed)) => {
                        each (parsed.fields.iter()) as field {
                            given argument_kind = intern(symbols, tree.source, argument_kind(&field.argument));
                            given mapping_key = intern(symbols, tree.source, mapping_key(&field.argument));
                            given conversion = intern_character(symbols, tree.source, Some(field.conversion));
                            given flags = intern(symbols, tree.source, languages::NameText(flag_text(field.flags).0));
                            given width = intern(symbols, tree.source, quantity_mode(field.width));
                            given precision = intern(symbols, tree.source, quantity_mode(field.precision));
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

/// The string tokens of the string at `node` into `tokens`, in the order they are written, each with the percent-format parse of
/// its body as it is written into `parses`: none for a token whose body cannot be told from its quotes. Nothing for a node that is
/// no string.
fn raw_segments(
    file: &SyntaxFile<'_>,
    node: AnyNodeRef<'_>,
    tokens: &mut Vec<languages::Position>,
    parses: &mut Vec<Option<FormatParseResult>>,
) {
    tokens.clear();
    parses.clear();
    let AnyNodeRef::ExprStringLiteral(literal) = node else {
        return;
    };
    for part in &literal.value {
        let Some(part_id) = id_of(part) else {
            continue;
        };
        for token in languages::node_table::children_of(file.table, part_id) {
            if kind_of(languages::node_table::kind_of(file.table, *token))
                != Kind::Token(Token::String)
            {
                continue;
            }
            let parsed = body(RawFormatToken(written(file, *token).0)).map(|raw| {
                RuffLiteralCFormat.parse(FormatInput(raw.0), FormatDialect::PercentString)
            });
            tokens.push(*token);
            parses.push(parsed);
        }
    }
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

// The engine fills `raw-format-segment` from the walked file: every row of the file, once.
ryo::fills! {
    RawFormatSegmentItem => languages::RawFormatSegment, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::RawFormatSegment as languages::CatalogRelation>::Row;
        let memo = &mut file.buffers.fixed;
        let syntax = file.syntax;
        let mut whole = source_whole(&syntax);
        let (mut tokens, mut parses) = (Vec::new(), Vec::new());
        for Numbered { id, node } in nodes_of_kind(&syntax, Kind::Node(Node::Constant)) {
            if !matches!(node, AnyNodeRef::ExprStringLiteral(_)) {
                continue;
            }
            raw_segments(&syntax, node, &mut tokens, &mut parses);
            if segments_whole(&parses) == languages::SyntaxCompleteness::Incomplete {
                whole = languages::SyntaxCompleteness::Incomplete;
            }
            for (ordinal, (token, parsed)) in tokens.iter().zip(&parses).enumerate() {
                let summary = summary_of(parsed.as_ref(), DynamicInput::NotObserved);
                let errors = error_parts(parsed.as_ref());
                let mut buffer = [0_u8; 4];
                let character = match errors.character {
                    Some(character) => &*character.0.encode_utf8(&mut buffer),
                    None => NO_CHARACTER,
                };
                rows.write(Row {
                    id: node_cell(&syntax, id),
                    segment: node_cell(&syntax, *token),
                    segment_ordinal: languages::Integer(i64::try_from(ordinal).unwrap_or(i64::MAX)),
                    dialect: fixed(words, memo, languages::NameText(PERCENT_STRING)),
                    outcome: fixed(words, memo, summary.outcome),
                    error_category: fixed(words, memo, errors.category),
                    error_character: word(words, languages::NameText(character)),
                    error_index: errors.index,
                    positional_consumption: languages::Integer(
                        i64::try_from(summary.summary.positional_consumption.0).unwrap_or(i64::MAX),
                    ),
                    mapping_present: property_flag(summary.summary.mapping_present),
                    dynamic_width: property_flag(summary.summary.dynamic_width),
                    dynamic_precision: property_flag(summary.summary.dynamic_precision),
                    file: syntax.file,
                    span: span_at(&syntax, *token),
                });
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
        let memo = &mut file.buffers.fixed;
        let syntax = file.syntax;
        let source = source_whole(&syntax);
        let mut whole = source;
        let (mut tokens, mut parses) = (Vec::new(), Vec::new());
        for Numbered { id, node } in nodes_of_kind(&syntax, Kind::Node(Node::Constant)) {
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
                        argument_kind: fixed(words, memo, argument_kind(&field.argument)),
                        mapping_key: word(words, mapping_key(&field.argument)),
                        conversion: word(words, languages::NameText(conversion)),
                        width_mode: fixed(words, memo, quantity_mode(field.width)),
                        precision_mode: fixed(words, memo, quantity_mode(field.precision)),
                        file: syntax.file,
                        span: span_at(&syntax, *token),
                        flags: fixed(words, memo, languages::NameText(flag_text(field.flags).0)),
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
