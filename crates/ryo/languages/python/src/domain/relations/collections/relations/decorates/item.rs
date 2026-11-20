//! The `decorates` relation of the language: each decorator of a function or a class, in its place, by the dotted name it writes or
//! calls and the arguments it passes; any other decorator has the unknown value of a dynamic decorator.

use crate::domain::extras::types::Extra;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::model::types::Declarations;
use crate::domain::name_facts::service::{decoration, decorator_ordinal as placed_among};
use crate::domain::names::service::unknown;
use crate::domain::names::service::{decorator_arguments, decorator_ordinal, decorator_target};
use crate::domain::names::types::Unresolved;
use crate::domain::node::service::{node_cell, nodes_of_kind, span_at, word};
use crate::domain::node::types::Numbered;
use crate::domain::syntax::service::{kind_bits, parent};
use crate::domain::syntax::types::Kind;
use ryo::languages;
use ryo::languages::node_table;

/// Reads the `decorates` rows of one Python file from its syntax tree.
pub(crate) struct DecoratesItem;

ryo::reads! {
    DecoratesItem => languages::Decorates, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Extra(Extra::Decorator)] as kind_bits,
        row {
            occurrence: node(at),
            declaration: node(parent(tree, at)),
            ordinal: value(decorator_ordinal(tree, at)),
            target: value(decorator_target(tree, symbols, at)),
            arguments: value(decorator_arguments(tree, symbols, at)),
            file: file,
            span: span(at),
        }
    }
}

// The engine fills `decorates` from the walked file: every row of the file, once.
ryo::fills! {
    DecoratesItem => languages::Decorates, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Decorates as languages::CatalogRelation>::Row;
        let names = &mut file.buffers.text;
        for Numbered { id, .. } in nodes_of_kind(&file.syntax, Kind::Extra(Extra::Decorator)) {
            let ordinal = placed_among(&file.syntax, id);
            if let Some(found) = decoration(&file.syntax, id, names) {
                rows.write(Row {
                    occurrence: node_cell(&file.syntax, id),
                    declaration: node_cell(&file.syntax, node_table::parent_of(file.syntax.table, id)),
                    ordinal,
                    target: found
                        .target
                        .map_or(unknown(Unresolved::DynamicDecorator), |name| word(words, name)),
                    arguments: word(words, found.arguments),
                    file: file.syntax.file,
                    span: span_at(&file.syntax, id),
                });
            }
        }
    }
}
