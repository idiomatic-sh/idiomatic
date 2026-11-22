//! Inject the brace grammar into a language fact reader, independently of lint call policy.
use super::constants::{DYNAMIC, INVALID_TYPE, OTHER_ERROR, PARTIAL_CAUSE, RECOGNIZED};
use crate::application::brace_format::service::prepare;
use crate::domain::brace_format::types::BraceFieldContext;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::format_specification::constants::{BRACE_STRING, FSTRING, NO_TEXT};
use crate::domain::format_specification::types::ObservationCompleteness;
use crate::domain::format_syntax::types::FormatCharacter;
use crate::domain::literal_facts::service::format_spec_value;
use crate::domain::literal_facts::types::StringValue;
use crate::domain::model::names::service::fixed_word;
use crate::domain::names::service::unknown;
use crate::domain::names::types::Unresolved;
use crate::domain::node::service::{element_id, node_at, node_cell, span_at, word};
use crate::domain::spec_facts::service::{owned_fields, parent_place};
use crate::domain::standard_format_spec::service::recognize;
use crate::domain::standard_format_spec::types::{FormatSpecText, StandardFormatTarget};
use crate::domain::syntax::service::kind_of;
use crate::domain::{
    brace_format::types::{BraceFormatResult, BraceFormatState, CachedBraceFormat},
    format_specification::{
        service::{observe, text_of},
        types::Observations,
    },
    format_syntax::types::RetainedFormatIndex,
    model::types::Declarations,
    nodes::types::Node,
    read_scratch::types::ReadScratch,
    semantic_syntax::types::Visibility,
    standard_format_spec::types::StandardSpecOutcome,
    standard_library::types::TargetVersion,
    syntax::{service::kind_bits, types::Kind},
};
use crate::inbound::contracts::parse::readers::brace_format::composition::{
    parent_ordinal, parsed_strings,
};
use crate::outbound::brace_format::adapters::ruff_literal_braceformat::grammar::RuffLiteralBraceFormat;
use ruff_python_ast::AnyNodeRef;
use ryo::languages::{self, node_table};

/// Reads the standard format specifications of whole string carriers.
pub(crate) struct FormatSpecificationItem;

/// Whether the parser decoded every input the specifications are read from.
fn completeness(observations: &Observations) -> ObservationCompleteness {
    observations.completeness
}

/// The observations of the file, in the model's retained buffers.
fn observed<'model>(
    tree: &languages::Tree<'_>,
    braces: &RetainedFormatIndex<BraceFormatResult, CachedBraceFormat, BraceFormatState>,
    target: TargetVersion,
    scratch: &'model std::cell::RefCell<ReadScratch>,
) -> std::cell::Ref<'model, Observations> {
    observe(
        tree,
        braces,
        target,
        &mut scratch.borrow_mut().format_observations,
    );
    std::cell::Ref::map(scratch.borrow(), |held| &held.format_observations)
}

/// The symbol of the presentation type an invalid specification names, or of no text for any other outcome.
fn invalid_type_symbol(
    symbols: &mut languages::Symbols,
    source: languages::ByteSlice<'_>,
    outcome: StandardSpecOutcome,
) -> languages::Symbol {
    let mut buffer = [0_u8; 4];
    let text = match outcome {
        StandardSpecOutcome::InvalidType(character) => {
            languages::NameText(&*character.0.encode_utf8(&mut buffer))
        }
        StandardSpecOutcome::Recognized
        | StandardSpecOutcome::Dynamic
        | StandardSpecOutcome::OtherError => languages::NameText::default(),
    };
    languages::intern_derived(symbols, source, text)
}

ryo::reads! {
    FormatSpecificationItem => languages::FormatSpecification, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits, expanded,
        given braces = prepare(&RuffLiteralBraceFormat, tree, &model.brace_format, symbols, crate::domain::semantic_syntax::service::positions(tree), |node, symbols| (crate::domain::semantic_syntax::service::visibility(tree, model, node, symbols) == Visibility::Visible).into());
        given observations = observed(tree,&braces,model.language_target,&model.scratch);
        each (observations.rows.iter()) as spec {
            require crate::domain::semantic_syntax::service::visibility(tree,model,spec.id,symbols) == Visibility::Visible;
            given ordinal = languages::Integer(i64::try_from(spec.ordinal.0).unwrap_or(i64::MAX));
            given parent_ordinal = languages::Integer(spec.parent.map_or(-1,|parent|i64::try_from(parent.0).unwrap_or(i64::MAX)));
            given source_kind = languages::intern_derived(symbols,tree.source,languages::NameText(spec.source_kind.0));
            given value = match &spec.value { Some(value) => languages::intern_derived(symbols,tree.source,text_of(&observations, value.0)), None => crate::domain::names::service::string_or_unknown(tree,symbols,spec.origin) };
            given label = match spec.outcome { StandardSpecOutcome::Recognized => RECOGNIZED, StandardSpecOutcome::Dynamic => DYNAMIC, StandardSpecOutcome::InvalidType(_) => INVALID_TYPE, StandardSpecOutcome::OtherError => OTHER_ERROR };
            given outcome = languages::intern_derived(symbols,tree.source,languages::NameText(label));
            given invalid_type = invalid_type_symbol(symbols, tree.source, spec.outcome);
            given selector = languages::intern_derived(symbols,tree.source,text_of(&observations, spec.selector.0));
            given conversion = languages::intern_derived(symbols,tree.source,text_of(&observations, spec.conversion.0));
            row { id: node(spec.id), ordinal: value(ordinal), parent_ordinal: value(parent_ordinal), origin: node(spec.origin), source_kind: value(source_kind), value: value(value), outcome: value(outcome), invalid_type: value(invalid_type), selector: value(selector), conversion: value(conversion), file: file, span: span(spec.origin), }
        }
        when completeness(&observations) {
            ObservationCompleteness::Partial => { partial PARTIAL_CAUSE => languages::Missing::Any; }
            ObservationCompleteness::Whole => {}
        }
    }
}

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
            if matches!(parsed, Some(BraceFormatResult::Unavailable)) {
                whole = languages::SyntaxCompleteness::Incomplete;
            }
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
                let Some(origin) = element_id(&syntax, &**spec) else {
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
