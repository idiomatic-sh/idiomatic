//! Read one classification row per original expression and union member.
use crate::domain::{
    expression_types::{
        service::prepare,
        types::{ExpressionTypeIndex, IndexState, Resolution},
    },
    model::types::Declarations,
    nodes::types::{Category, Node},
    semantic_syntax::types::Visibility,
    syntax::{
        service::{category, kind_bits},
        types::Kind,
    },
};
use ryo::languages;

/// The reason incomplete parser payloads cannot establish basic types.
const INCOMPLETE: &str = "basic expression types require complete parser payloads";
/// Reads inferred expression classifications on demand.
pub(crate) struct ExpressionTypeItem;

/// Return whether the cached classifications cover the current source.
fn state(index: &ExpressionTypeIndex) -> IndexState {
    index.state
}

/// Look up the classification of one original expression position.
fn classification(
    index: &ExpressionTypeIndex,
    position: languages::Position,
) -> Option<&Resolution> {
    index
        .entries
        .get(usize::try_from(position.0).ok()?)
        .and_then(Option::as_ref)
}

ryo::reads! {
    ExpressionTypeItem => languages::ExpressionType, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        given index = prepare(tree, model);
        when state(&index) {
            IndexState::Partial => { partial INCOMPLETE => languages::Missing::Any; }
            IndexState::Fresh => { partial INCOMPLETE => languages::Missing::Any; }
            IndexState::Complete => {}
        }
        each (crate::domain::semantic_syntax::service::positions(tree)) as expression {
            require category(tree, expression) == Category::Expression;
            require crate::domain::semantic_syntax::service::visibility(tree, model, expression, symbols) == Visibility::Visible;
            when classification(&index, expression) {
                Some(Resolution::Atom(type_name)) => {
                    given resolution = languages::intern_derived(symbols, tree.source, languages::NameText("atom"));
                    given kind = languages::intern_derived(symbols, tree.source, languages::NameText(type_name));
                    row { id: node(expression), resolution: value(resolution), type_: value(kind), file: file, span: span(expression), }
                }
                Some(Resolution::Union(types)) => {
                    given resolution = languages::intern_derived(symbols, tree.source, languages::NameText("union"));
                    each (types.iter()) as type_name {
                        given kind = languages::intern_derived(symbols, tree.source, languages::NameText(type_name));
                        row { id: node(expression), resolution: value(resolution), type_: value(kind), file: file, span: span(expression), }
                    }
                }
                Some(Resolution::Unknown) => {
                    given resolution = languages::intern_derived(symbols, tree.source, languages::NameText("unknown"));
                    given kind = languages::intern_derived(symbols, tree.source, languages::NameText(""));
                    row { id: node(expression), resolution: value(resolution), type_: value(kind), file: file, span: span(expression), }
                }
                Some(Resolution::TypeError) => {
                    given resolution = languages::intern_derived(symbols, tree.source, languages::NameText("type-error"));
                    given kind = languages::intern_derived(symbols, tree.source, languages::NameText(""));
                    row { id: node(expression), resolution: value(resolution), type_: value(kind), file: file, span: span(expression), }
                }
                None => {}
            }
        }
    }
}
