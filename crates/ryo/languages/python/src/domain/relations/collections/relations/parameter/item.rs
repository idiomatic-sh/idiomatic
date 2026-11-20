//! The `parameter` relation of the language: each parameter of a function or a lambda, in its place, by how it takes its argument;
//! the markers `/` and `*` are no parameters.

use crate::domain::definitions::service::{placed, word};
use crate::domain::extras::types::Extra;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::model::types::Declarations;
use crate::domain::name_facts::service::parameter_place;
use crate::domain::node::service::{fixed, node_cell, nodes_of_kind, span_at, word as interned};
use crate::domain::node::types::Numbered;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{child, kind_bits, name_of};
use crate::domain::syntax::types::Kind;
use ruff_python_ast::AnyNodeRef;
use ryo::languages;

/// Reads the `parameter` rows of one Python file from its syntax tree.
pub(crate) struct ParameterItem;

ryo::reads! {
    ParameterItem => languages::Parameter, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Arg)] as kind_bits,
        given place = placed(tree, at)?;
        given identifier = child(tree, at, Kind::Extra(Extra::Identifier))?;
        given name = name_of(tree, identifier)?;
        row {
            id: node(at),
            kind: value(place.kind),
            refinement: value(word(tree, symbols, place.word)),
            name: value(name),
            ordinal: value(place.ordinal),
            owner: node(place.callable),
            file: file,
            span: span(at),
        }
    }
}

// The engine fills `parameter` from the walked file: every row of the file, once.
ryo::fills! {
    ParameterItem => languages::Parameter, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Parameter as languages::CatalogRelation>::Row;
        let memo = &mut file.buffers.fixed;
        for Numbered { id, node } in nodes_of_kind(&file.syntax, Kind::Node(Node::Arg)) {
            let AnyNodeRef::Parameter(parameter) = node else {
                continue;
            };
            let Some(place) = parameter_place(&file.syntax, id) else {
                continue;
            };
            if parameter.name.id.is_empty() {
                continue;
            }
            rows.write(Row {
                id: node_cell(&file.syntax, id),
                kind: place.kind,
                refinement: fixed(words, memo, languages::NameText(place.word.0)),
                name: interned(words, languages::NameText(parameter.name.id.as_str())),
                ordinal: place.ordinal,
                owner: node_cell(&file.syntax, place.callable),
                file: file.syntax.file,
                span: span_at(&file.syntax, id),
            });
        }
    }
}
