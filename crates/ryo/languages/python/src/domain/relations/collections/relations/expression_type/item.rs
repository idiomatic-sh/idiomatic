//! Read one classification row per original expression and union member.
use crate::domain::file_view::types::{PythonFile, PythonWalk};
use crate::domain::model::names::service::fixed_word;
use crate::domain::model::names::types::FixedWords;
use crate::domain::model::view::types::PartState;
use crate::domain::node::service::{keyed_node, node_cell, span_at, typed_nodes};
use crate::domain::node::types::{Numbered, SyntaxFile};
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

/// Says the relation is read only in part when the parser did not read the whole file: a type may be missing.
fn told_incomplete(
    file: &PythonFile<'_>,
    rows: &mut dyn languages::Rows<languages::ExpressionTypeRow>,
) {
    if file.syntax.completeness.syntax == languages::SyntaxCompleteness::Incomplete {
        rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
    }
}

/// Writes the rows of the node `id`, whose basic type is `found`: one for each type it may be; none for a node that is no
/// expression.
fn type_rows(
    file: &SyntaxFile<'_>,
    words: &mut languages::FileWords,
    memo: &mut FixedWords,
    id: languages::Position,
    found: &Resolution,
    rows: &mut dyn languages::Rows<languages::ExpressionTypeRow>,
) {
    if category_of(kind_of(node_table::kind_of(file.table, id))) != Category::Expression {
        return;
    }
    let mut write = |resolution: languages::NameText<'static>,
                     kind: languages::NameText<'static>| {
        rows.write(languages::ExpressionTypeRow {
            id: node_cell(file, id),
            resolution: fixed_word(words, memo, resolution),
            type_: fixed_word(words, memo, kind),
            file: file.file,
            span: span_at(file, id),
        });
    };
    let name = crate::domain::expression_types::constants::name;
    let (label, none) = (
        languages::NameText,
        languages::NameText(super::constants::NO_TYPE),
    );
    match found {
        Resolution::Atom(kind) => write(label(super::constants::ATOM), name(*kind)),
        Resolution::Union(kinds) => {
            for kind in kinds {
                write(label(super::constants::UNION), name(*kind));
            }
        }
        Resolution::Unknown => write(label(super::constants::UNKNOWN), none),
        Resolution::TypeError => write(label(super::constants::TYPE_ERROR), none),
    }
}

/// Finds the basic type of the file's nodes, when no fill of the file has yet: an operation's type is read from its operands',
/// so the types are found whole, once, by the first fill that asks, whole or by key.
fn typed(file: &mut PythonFile<'_>) {
    if file.analysed.types == PartState::Pending {
        resolutions(&file.syntax, &mut file.buffers.types);
        file.analysed.types = PartState::Ready;
    }
}

// The engine fills `expression-type` from the walked file: every row of the file, or the rows of one expression, read from the
// type the file's types give it.
ryo::fills! {
    ExpressionTypeItem => languages::ExpressionType, walk(PythonWalk) |file, words, rows| {
        told_incomplete(file, rows);
        typed(file);
        let memo = &mut file.model.words.get_mut().fixed;
        for (Numbered { id, .. }, found) in typed_nodes(&file.syntax).zip(file.buffers.types.iter()) {
            type_rows(&file.syntax, words, memo, id, found, rows);
        }
    }
    key(languages::Column::ExpressionType(languages::ExpressionTypeColumn::Id)) |file, words, key, rows| {
        told_incomplete(file, rows);
        typed(file);
        let Some(id) = keyed_node(&file.syntax, key) else {
            return;
        };
        if let Some(found) = file.buffers.types.get(usize::from(id)) {
            type_rows(&file.syntax, words, &mut file.model.words.get_mut().fixed, id, found, rows);
        }
    }
}
