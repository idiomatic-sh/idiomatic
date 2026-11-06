//! Inject the brace grammar into a language fact reader, independently of lint call policy.
use super::constants::{DYNAMIC, INVALID_TYPE, OTHER_ERROR, PARTIAL_CAUSE, RECOGNIZED};
use crate::domain::{
    brace_format::service::prepare,
    format_specification::service::observe,
    model::types::Declarations,
    nodes::types::Node,
    semantic_syntax::types::Visibility,
    standard_format_spec::types::StandardSpecOutcome,
    syntax::{service::kind_bits, types::Kind},
};
use crate::outbound::format_syntax::adapters::ruff_literal_braceformat::grammar::RuffLiteralBraceFormat;
use ryo::languages;

/// Reads the standard format specifications of whole string carriers.
pub(crate) struct FormatSpecificationItem;

/// Whether the parser decoded every input the specifications are read from.
enum Completeness {
    Whole,
    Partial,
}

fn completeness(
    observations: &crate::domain::format_specification::types::Observations,
) -> Completeness {
    if observations.partial {
        Completeness::Partial
    } else {
        Completeness::Whole
    }
}

ryo::reads! {
    FormatSpecificationItem => languages::FormatSpecification, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        given braces = prepare::<RuffLiteralBraceFormat>(tree, &model.brace_format, symbols, crate::domain::semantic_syntax::service::positions(tree), |node, symbols| (crate::domain::semantic_syntax::service::visibility(tree, model, node, symbols) == crate::domain::semantic_syntax::types::Visibility::Visible).into());
        given observations = observe(tree,&braces,model.language_target);
        each (observations.rows.iter()) as spec {
            require crate::domain::semantic_syntax::service::visibility(tree,model,spec.id,symbols) == Visibility::Visible;
            given ordinal = languages::Integer(i64::try_from(spec.ordinal).unwrap_or(i64::MAX));
            given parent_ordinal = languages::Integer(spec.parent.map_or(-1,|parent|i64::try_from(parent).unwrap_or(i64::MAX)));
            given source_kind = languages::intern_derived(symbols,tree.source,languages::NameText(spec.source_kind));
            given value = match &spec.value { Some(value) => languages::intern_derived(symbols,tree.source,languages::NameText(value)), None => crate::domain::names::service::string_or_unknown(tree,symbols,spec.origin) };
            given label = match spec.outcome { StandardSpecOutcome::Recognized => RECOGNIZED, StandardSpecOutcome::Dynamic => DYNAMIC, StandardSpecOutcome::InvalidType(_) => INVALID_TYPE, StandardSpecOutcome::OtherError => OTHER_ERROR };
            given outcome = languages::intern_derived(symbols,tree.source,languages::NameText(label));
            given invalid = match spec.outcome { StandardSpecOutcome::InvalidType(character) => character.to_string(), _ => String::new() };
            given invalid_type = languages::intern_derived(symbols,tree.source,languages::NameText(&invalid));
            given selector = languages::intern_derived(symbols,tree.source,languages::NameText(&spec.selector));
            given conversion = languages::intern_derived(symbols,tree.source,languages::NameText(&spec.conversion));
            row { id: node(spec.id), ordinal: value(ordinal), parent_ordinal: value(parent_ordinal), origin: node(spec.origin), source_kind: value(source_kind), value: value(value), outcome: value(outcome), invalid_type: value(invalid_type), selector: value(selector), conversion: value(conversion), file: file, span: span(spec.origin), }
        }
        when completeness(&observations) {
            Completeness::Partial => { partial PARTIAL_CAUSE => languages::Missing::Any; }
            Completeness::Whole => {}
        }
    }
}
