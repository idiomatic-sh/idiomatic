//! Inject the brace grammar into a language fact reader, independently of lint call policy.
use super::constants::{DYNAMIC, INVALID_TYPE, OTHER_ERROR, PARTIAL_CAUSE, RECOGNIZED};
use crate::application::brace_format::service::prepare;
use crate::domain::format_specification::types::ObservationCompleteness;
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
use crate::outbound::brace_format::adapters::ruff_literal_braceformat::grammar::RuffLiteralBraceFormat;
use ryo::languages;

/// Reads the standard format specifications of whole string carriers.
pub(crate) struct FormatSpecificationItem;

/// Whether the parser decoded every input the specifications are read from.
fn completeness(
    observations: &crate::domain::format_specification::types::Observations,
) -> ObservationCompleteness {
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
        given braces = prepare(&RuffLiteralBraceFormat, tree, &model.brace_format, symbols, crate::domain::semantic_syntax::service::positions(tree), |node, symbols| (crate::domain::semantic_syntax::service::visibility(tree, model, node, symbols) == crate::domain::semantic_syntax::types::Visibility::Visible).into());
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
