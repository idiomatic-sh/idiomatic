//! Inject the brace grammar into a language fact reader, independently of lint call policy.
use super::constants::{DYNAMIC, INVALID_TYPE, OTHER_ERROR, PARTIAL_CAUSE, RECOGNIZED};

use crate::domain::brace_format::types::BraceFieldContext;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::format_specification::constants::{BRACE_STRING, FSTRING, NO_TEXT};

use crate::domain::format_syntax::types::FormatCharacter;
use crate::domain::literal_facts::service::format_spec_value;
use crate::domain::literal_facts::types::StringValue;
use crate::domain::model::names::service::fixed_word;
use crate::domain::names::service::unknown;
use crate::domain::names::types::Unresolved;
use crate::domain::node::service::{id_of, node_at, node_cell, span_at, word};
use crate::domain::spec_facts::service::{owned_fields, parent_place};
use crate::domain::standard_format_spec::service::recognize;
use crate::domain::standard_format_spec::types::{FormatSpecText, StandardFormatTarget};
use crate::domain::syntax::service::kind_of;
use crate::domain::{brace_format::types::{BraceFormatResult}, nodes::types::Node, standard_format_spec::types::StandardSpecOutcome, syntax::{types::Kind}};
use crate::inbound::contracts::parse::readers::brace_format::composition::{parent_ordinal, parsed_strings};

use ruff_python_ast::AnyNodeRef;
use ryo::languages::{self, node_table};

/// Reads the standard format specifications of whole string carriers.
pub(crate) struct FormatSpecificationItem;

/// The label of an outcome, as the relation names it.
const fn label(outcome: StandardSpecOutcome) -> languages::NameText<'static> {
    languages::NameText(match outcome {
        StandardSpecOutcome::Recognized => RECOGNIZED,
        StandardSpecOutcome::Dynamic => DYNAMIC,
        StandardSpecOutcome::InvalidType(_) => INVALID_TYPE,
        StandardSpecOutcome::OtherError => OTHER_ERROR,
    })
}

/// A conversion as a word of the file; the word of no text for none.
fn conversion_word(
    words: &mut languages::FileWords,
    conversion: Option<FormatCharacter>,
) -> languages::Symbol {
    let mut buffer = [0_u8; 4];
    word(
        words,
        conversion.map_or(languages::NameText(NO_TEXT), |each| {
            languages::NameText(&*each.0.encode_utf8(&mut buffer))
        }),
    )
}

/// The presentation type an invalid specification names, as a word of the file; the word of no text for any other outcome.
fn invalid_type_word(
    words: &mut languages::FileWords,
    outcome: StandardSpecOutcome,
) -> languages::Symbol {
    let mut buffer = [0_u8; 4];
    word(
        words,
        match outcome {
            StandardSpecOutcome::InvalidType(character) => {
                languages::NameText(&*character.0.encode_utf8(&mut buffer))
            }
            StandardSpecOutcome::Recognized
            | StandardSpecOutcome::Dynamic
            | StandardSpecOutcome::OtherError => languages::NameText(NO_TEXT),
        },
    )
}

// The engine fills `format-specification` from the walked file: every row of the file, once.
ryo::fills! {
    FormatSpecificationItem => languages::FormatSpecification, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::FormatSpecification as languages::CatalogRelation>::Row;
        let memo = &mut file.model.words.get_mut().fixed;
        let syntax = file.syntax;
        let target = StandardFormatTarget::from(file.target);
        let mut whole = languages::SyntaxCompleteness::Complete;
        // The specifications of the replacement fields of a string: an f-string's have their own syntax, read below.
        parsed_strings(&syntax, &mut file.buffers.text, &mut |id, parsed, _unknown| {
            if kind_of(node_table::kind_of(syntax.table, id)) != Kind::Node(Node::Constant) {
                return;
            }
            let Some(BraceFormatResult::Parsed(parsed)) = parsed else {
                return;
            };
            for field in &parsed.fields {
                let Some(spec) = &field.format_spec else {
                    continue;
                };
                let nested = parsed.fields.iter().any(|each| {
                    matches!(each.context, BraceFieldContext::FormatSpec { parent_ordinal } if parent_ordinal == field.ordinal)
                });
                let outcome = recognize(FormatSpecText(&spec.0), nested.into(), target);
                rows.write(Row {
                    id: node_cell(&syntax, id),
                    ordinal: languages::Integer(i64::try_from(field.ordinal.0).unwrap_or(i64::MAX)),
                    parent_ordinal: parent_ordinal(&field.context),
                    origin: node_cell(&syntax, id),
                    source_kind: fixed_word(words, memo, languages::NameText(BRACE_STRING)),
                    value: word(words, languages::NameText(&spec.0)),
                    outcome: fixed_word(words, memo, label(outcome)),
                    invalid_type: invalid_type_word(words, outcome),
                    selector: word(words, languages::NameText(&field.selector_text.0)),
                    conversion: conversion_word(words, field.conversion),
                    file: syntax.file,
                    span: span_at(&syntax, id),
                });
            }
        });
        // The specifications of the interpolations of an f-string, numbered among the interpolations of their f-string.
        let owned = &mut file.buffers.owned;
        owned_fields(&syntax, owned);
        for group in owned.chunk_by(|left, right| left.owner == right.owner) {
            for (ordinal, each) in group.iter().enumerate() {
                let Some(AnyNodeRef::InterpolatedElement(interpolation)) = node_at(&syntax, each.field) else {
                    continue;
                };
                let Some(spec) = &interpolation.format_spec else {
                    continue;
                };
                let Some(origin) = id_of(&**spec) else {
                    continue;
                };
                let (value, outcome) = match format_spec_value(AnyNodeRef::from(&**spec), &mut file.buffers.text) {
                    Some(StringValue::Known(text)) => (
                        word(words, text),
                        recognize(FormatSpecText(text.0), false.into(), target),
                    ),
                    Some(StringValue::Interpolated) => (
                        unknown(Unresolved::InterpolatedString),
                        StandardSpecOutcome::Dynamic,
                    ),
                    None => continue,
                };
                let parent = parent_place(&syntax, group, *each);
                rows.write(Row {
                    id: node_cell(&syntax, each.owner),
                    ordinal: languages::Integer(i64::try_from(ordinal).unwrap_or(i64::MAX)),
                    parent_ordinal: languages::Integer(parent.map_or(-1, |place| i64::try_from(place.0).unwrap_or(i64::MAX))),
                    origin: node_cell(&syntax, origin),
                    source_kind: fixed_word(words, memo, languages::NameText(FSTRING)),
                    value,
                    outcome: fixed_word(words, memo, label(outcome)),
                    invalid_type: invalid_type_word(words, outcome),
                    selector: fixed_word(words, memo, languages::NameText(NO_TEXT)),
                    conversion: conversion_word(words, interpolation.conversion.to_char().map(FormatCharacter)),
                    file: syntax.file,
                    span: span_at(&syntax, origin),
                });
            }
        }
        if whole == languages::SyntaxCompleteness::Incomplete {
            rows.partial(languages::ReportText(PARTIAL_CAUSE), languages::Missing::Any);
        }
    }
}
