//! The `call_argument` relation of the language: each argument of a call, by its place among the arguments. A call of a type written
//! out converts, and has no row, as it has none of `static_call`.

use crate::domain::names::service::{arguments_of, callee_of, passed};
use crate::domain::names::types::Callee;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_bits, nodes};
use crate::domain::syntax::types::Kind;
use ryo::languages;

use crate::domain::file_view::types::{GoWalk, Words};
use crate::domain::walked::syntax as walked_syntax;
use ryo::languages::node_table;

/// The reader the `parse` contract hands the `call_argument` relation to, which reads its rows from a file's tree.
pub(crate) struct CallArgumentItem;

ryo::reads! {
    CallArgumentItem => languages::CallArgument |tree, at, symbols| {
        kinds: [Kind::Node(Node::CallExpr)] as kind_bits,
        given callee = nodes(tree, at).next()?;
        require callee_of(tree, callee) != Callee::Conversion;
        each (arguments_of(tree, at)) as (position, argument) {
            row {
                call: node(at),
                position: value(languages::Integer(i64::from(position.0))),
                expression: node(argument),
                passes: value(passed(tree, argument)),
                file: file,
                span: span(argument),
            }
        }
    }
}

// The engine fills `call_argument` from the walked file: each argument of a call that is no conversion, with the name it hands on.
ryo::fills! {
    CallArgumentItem => languages::CallArgument, walk(GoWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::CallArgument as languages::CatalogRelation>::Row;
        let tree = file.syntax;
        let words = Words(std::cell::RefCell::new(words));
        for &at in node_table::ids_of_kind(tree.table, kind_bits(Kind::Node(Node::CallExpr))) {
            let Some(callee) = crate::domain::walked::syntax::nodes(&tree, at).next() else { continue };
            if crate::domain::walked::names::callee_of(&tree, callee) == Callee::Conversion {
                continue;
            }
            for (position, argument) in crate::domain::walked::names::arguments_of(&tree, at).enumerate() {
                rows.write(Row {
                    call: walked_syntax::node_cell(&tree, at),
                    position: languages::Integer(i64::try_from(position).unwrap_or(i64::MAX)),
                    expression: walked_syntax::node_cell(&tree, argument),
                    passes: crate::domain::walked::names::passed(&tree, &words, argument),
                    file: tree.file,
                    span: walked_syntax::span_of(&tree, argument),
                });
            }
        }
    }
}
