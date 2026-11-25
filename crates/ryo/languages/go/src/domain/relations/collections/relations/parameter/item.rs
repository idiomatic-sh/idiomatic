//! The `parameter` relation of the language: the receiver of a method, each parameter of a function, a method or a function literal,
//! and each of its named results, each in its place in its list. A parameter the declaration does not name has the unknown value of
//! that for its name.

use crate::domain::nodes::types::Node;
use crate::domain::signatures::service::parameters;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

use crate::domain::file_view::types::{GoWalk, Words};
use crate::domain::walked::syntax as walked_syntax;
use ryo::languages::node_table;

/// The reader the `parse` contract hands the `parameter` relation to, which reads its rows from a file's tree.
pub(crate) struct ParameterItem;

ryo::reads! {
    ParameterItem => languages::Parameter |tree, at, symbols| {
        kinds: [Kind::Node(Node::Field)] as kind_bits,
        each (parameters(tree, at)) as parameter {
            given refinement = languages::intern_derived(symbols, tree.source, languages::NameText(parameter.word.0));
            row {
                id: node(parameter.named),
                kind: value(parameter.kind),
                refinement: value(refinement),
                name: value(parameter.name),
                ordinal: value(parameter.ordinal),
                owner: node(parameter.callable),
                file: file,
                span: span(parameter.named),
            }
        }
    }
}

// The engine fills `parameter` from the walked file: the receiver, each parameter and each named result of every field list.
ryo::fills! {
    ParameterItem => languages::Parameter, walk(GoWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Parameter as languages::CatalogRelation>::Row;
        let tree = file.syntax;
        let words = Words(std::cell::RefCell::new(words));
        for &at in node_table::ids_of_kind(tree.table, kind_bits(Kind::Node(Node::Field))) {
            for parameter in crate::domain::walked::signatures::parameters(&tree, &words, at) {
                let refinement = walked_syntax::word(&words, parameter.word.0);
                rows.write(Row {
                    id: walked_syntax::node_cell(&tree, parameter.named),
                    kind: parameter.kind,
                    refinement,
                    name: parameter.name,
                    ordinal: parameter.ordinal,
                    owner: walked_syntax::node_cell(&tree, parameter.callable),
                    file: tree.file,
                    span: walked_syntax::span_of(&tree, parameter.named),
                });
            }
        }
    }
}
