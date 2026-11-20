//! Read one classification row per original expression and union member.
use crate::domain::file_view::types::PythonWalk;
use crate::domain::node::service::{fixed, node_cell, span_at, typed_nodes};
use crate::domain::node::types::Numbered;
use crate::domain::syntax::service::{category_of, kind_of};
use crate::domain::type_facts::service::resolutions;
use crate::domain::{
    expression_types::{
        service::prepare,
        types::{ExpressionTypeIndex, ExpressionTypeState, Resolution},
    },
    model::types::Declarations,
    nodes::types::{Category, Node},
    semantic_syntax::types::Visibility,
    syntax::{
        service::{category, kind_bits},
        types::Kind,
    },
};
use ryo::languages::{self, node_table};

/// The reason incomplete parser payloads cannot establish basic types.
use super::constants::INCOMPLETE;
/// Reads inferred expression classifications on demand.
pub(crate) struct ExpressionTypeItem;

/// Return whether the cached classifications cover the current source.
fn state(index: &ExpressionTypeIndex) -> ExpressionTypeState {
    index.state
}

/// Look up the classification of one original expression position.
fn classification(
    index: &ExpressionTypeIndex,
    position: languages::Position,
) -> Option<&Resolution> {
    let Ok(slot) = usize::try_from(position.0) else {
        // A nonrepresentable position cannot identify a resident classification.
        return None;
    };
    index.entries.get(slot).and_then(Option::as_ref)
}

ryo::reads! {
    ExpressionTypeItem => languages::ExpressionType, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits, expanded,
        given index = prepare(tree, &model.expression_types);
        when state(&index) {
            ExpressionTypeState::Partial => { partial INCOMPLETE => languages::Missing::Any; }
            ExpressionTypeState::Fresh => { partial INCOMPLETE => languages::Missing::Any; }
            ExpressionTypeState::Complete => {}
        }
        each (crate::domain::semantic_syntax::service::positions(tree)) as expression {
            require category(tree, expression) == Category::Expression;
            require crate::domain::semantic_syntax::service::visibility(tree, model, expression, symbols) == Visibility::Visible;
            when classification(&index, expression) {
                Some(Resolution::Atom(type_name)) => {
                    given resolution = languages::intern_derived(symbols, tree.source, languages::NameText(super::constants::ATOM));
                    given kind = languages::intern_derived(symbols, tree.source, crate::domain::expression_types::constants::name(*type_name));
                    row { id: node(expression), resolution: value(resolution), type_: value(kind), file: file, span: span(expression), }
                }
                Some(Resolution::Union(types)) => {
                    given resolution = languages::intern_derived(symbols, tree.source, languages::NameText(super::constants::UNION));
                    each (types.iter()) as type_name {
                        given kind = languages::intern_derived(symbols, tree.source, crate::domain::expression_types::constants::name(*type_name));
                        row { id: node(expression), resolution: value(resolution), type_: value(kind), file: file, span: span(expression), }
                    }
                }
                Some(Resolution::Unknown) => {
                    given resolution = languages::intern_derived(symbols, tree.source, languages::NameText(super::constants::UNKNOWN));
                    given kind = languages::intern_derived(symbols, tree.source, languages::NameText(super::constants::NO_TYPE));
                    row { id: node(expression), resolution: value(resolution), type_: value(kind), file: file, span: span(expression), }
                }
                Some(Resolution::TypeError) => {
                    given resolution = languages::intern_derived(symbols, tree.source, languages::NameText(super::constants::TYPE_ERROR));
                    given kind = languages::intern_derived(symbols, tree.source, languages::NameText(super::constants::NO_TYPE));
                    row { id: node(expression), resolution: value(resolution), type_: value(kind), file: file, span: span(expression), }
                }
                None => {}
            }
        }
    }
}

// The engine fills `expression-type` from the walked file: every row of the file, once.
ryo::fills! {
    ExpressionTypeItem => languages::ExpressionType, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::ExpressionType as languages::CatalogRelation>::Row;
        if file.syntax.completeness.syntax == languages::SyntaxCompleteness::Incomplete {
            rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
        }
        let memo = &mut file.buffers.fixed;
        let types = &mut file.buffers.types;
        resolutions(&file.syntax, types);
        for (Numbered { id, .. }, found) in typed_nodes(&file.syntax).zip(types.iter()) {
            if category_of(kind_of(node_table::kind_of(file.syntax.table, id))) != Category::Expression {
                continue;
            }
            let mut write = |resolution: &'static str, kind: languages::NameText<'static>| {
                rows.write(Row {
                    id: node_cell(&file.syntax, id),
                    resolution: fixed(words, memo, languages::NameText(resolution)),
                    type_: fixed(words, memo, kind),
                    file: file.syntax.file,
                    span: span_at(&file.syntax, id),
                });
            };
            match found {
                Resolution::Atom(kind) => write(super::constants::ATOM, crate::domain::expression_types::constants::name(*kind)),
                Resolution::Union(kinds) => {
                    for kind in kinds {
                        write(super::constants::UNION, crate::domain::expression_types::constants::name(*kind));
                    }
                }
                Resolution::Unknown => write(super::constants::UNKNOWN, languages::NameText(super::constants::NO_TYPE)),
                Resolution::TypeError => write(super::constants::TYPE_ERROR, languages::NameText(super::constants::NO_TYPE)),
            }
        }
    }
}
