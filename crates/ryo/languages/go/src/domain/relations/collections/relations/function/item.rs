//! The `function` relation of the language: every `func` declaration at any depth, methods included; a function literal is no function.

use crate::domain::names::service::name_of;
use crate::domain::nodes::types::Node;
use crate::domain::signatures::service::signature;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

use crate::domain::file_view::types::{GoWalk, Words};
use crate::domain::walked::syntax as walked_syntax;
use ryo::languages::node_table;

/// The reader the `parse` contract hands the `function` relation to, which reads its rows from a file's tree.
pub(crate) struct FunctionItem;

ryo::reads! {
    FunctionItem => languages::Function |tree, at, _symbols| {
        kinds: [Kind::Node(Node::FuncDecl)] as kind_bits,
        given identifier = signature(tree, at).name?;
        given name = name_of(tree, identifier)?;
        row { id: node(at), name: value(name), file: file, span: span(at) }
    }
}

// The engine fills `function` from the walked file: each function and method that has a name.
ryo::fills! {
    FunctionItem => languages::Function, walk(GoWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Function as languages::CatalogRelation>::Row;
        let tree = file.syntax;
        let words = Words(std::cell::RefCell::new(words));
        for &at in node_table::ids_of_kind(tree.table, kind_bits(Kind::Node(Node::FuncDecl))) {
            let Some(identifier) = crate::domain::walked::signatures::signature(&tree, at).name else { continue };
            let Some(name) = crate::domain::walked::names::name_of(&tree, &words, identifier) else { continue };
            rows.write(Row {
                id: walked_syntax::node_cell(&tree, at),
                name,
                file: tree.file,
                span: walked_syntax::span_of(&tree, at),
            });
        }
    }
}
