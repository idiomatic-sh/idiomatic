//! The `scope` relation of the language: the file, and each function, method and function literal, each under the scope that
//! encloses it, the file's for the outermost; the file encloses itself. A `go.mod` is a file of the module too, and encloses itself.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;

use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

use crate::domain::file_view::types::GoWalk;
use crate::domain::walked::syntax as walked_syntax;
use ryo::languages::node_table;

/// The reader the `parse` contract hands the `scope` relation to, which reads its rows from a file's tree.
pub(crate) struct ScopeItem;

// The engine fills `scope` from the walked file: the file and each function, method and function literal, under the scope that encloses it.
ryo::fills! {
    ScopeItem => languages::Scope, walk(GoWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Scope as languages::CatalogRelation>::Row;
        let _ = words;
        let tree = file.syntax;
        for wanted in [Kind::Node(Node::File), Kind::Extra(Extra::ModFile), Kind::Node(Node::FuncDecl), Kind::Node(Node::FuncLit)] {
            for &at in node_table::ids_of_kind(tree.table, kind_bits(wanted)) {
                rows.write(Row {
                    parent: crate::domain::walked::scopes::scope_owner(&tree, at),
                    child: walked_syntax::node_cell(&tree, at),
                    file: tree.file,
                    span: walked_syntax::span_of(&tree, at),
                });
            }
        }
    }
}
