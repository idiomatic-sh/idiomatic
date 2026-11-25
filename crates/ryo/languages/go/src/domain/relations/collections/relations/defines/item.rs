//! The `defines` relation of the language: every name a file declares or brings in: functions and methods, types, constants and
//! variables, imported names, fields and methods of types, parameters, results and receivers, type parameters, and the names a short
//! variable declaration or a `range` clause declares, each in the function that holds it. The blank identifier declares nothing. An
//! import of `.` brings in names it cannot list: its name is the unknown value of a dot import.

use crate::domain::definitions::service::{declarations, declared_name, word};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

use crate::domain::file_view::types::{GoWalk, Words};
use crate::domain::walked::syntax as walked_syntax;
use ryo::languages::node_table;

/// The reader the `parse` contract hands the `defines` relation to, which reads its rows from a file's tree.
pub(crate) struct DefinesItem;

ryo::reads! {
    DefinesItem => languages::Defines |tree, at, symbols| {
        kinds: [
            Kind::Node(Node::FuncDecl),
            Kind::Node(Node::TypeSpec),
            Kind::Node(Node::ValueSpec),
            Kind::Node(Node::ImportSpec),
            Kind::Node(Node::Field),
            Kind::Node(Node::AssignStmt),
            Kind::Node(Node::RangeStmt),
            Kind::Node(Node::LabeledStmt),
        ] as kind_bits,
        each (declarations(tree, at)) as declared {
            given name = declared_name(tree, &declared)?;
            row {
                id: node(at),
                kind: value(declared.defined.kind),
                refinement: value(word(tree, symbols, declared.defined.word)),
                name: value(name),
                owner: node(declared.owner),
                file: file,
                span: span(declared.named),
            }
        }
    }
}

// The engine fills `defines` from the walked file: every name the file declares or brings in, each in the function that holds it.
ryo::fills! {
    DefinesItem => languages::Defines, walk(GoWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Defines as languages::CatalogRelation>::Row;
        let tree = file.syntax;
        let words = Words(std::cell::RefCell::new(words));
        for wanted in [
            Kind::Node(Node::FuncDecl),
            Kind::Node(Node::TypeSpec),
            Kind::Node(Node::ValueSpec),
            Kind::Node(Node::ImportSpec),
            Kind::Node(Node::Field),
            Kind::Node(Node::AssignStmt),
            Kind::Node(Node::RangeStmt),
            Kind::Node(Node::LabeledStmt),
        ] {
            for &at in node_table::ids_of_kind(tree.table, kind_bits(wanted)) {
                for declared in crate::domain::walked::definitions::declarations(&tree, at) {
                    let Some(name) = crate::domain::walked::definitions::declared_name(&tree, &words, &declared) else { continue };
                    rows.write(Row {
                        id: walked_syntax::node_cell(&tree, at),
                        kind: declared.defined.kind,
                        refinement: crate::domain::walked::definitions::word(&words, declared.defined.word),
                        name,
                        owner: walked_syntax::node_cell(&tree, declared.owner),
                        file: tree.file,
                        span: walked_syntax::span_of(&tree, declared.named),
                    });
                }
            }
        }
    }
}
