//! The `assigns` relation of the language: the string whose value is known that `=`, `:=`, `var` or `const` binds to a name, linked to
//! the name and the string, owned by the function it is in or the file's start. Each name is paired with the value at its place.


use crate::domain::nodes::types::Node;

use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

use crate::domain::file_view::types::GoWalk;
use crate::domain::walked::syntax as walked_syntax;
use ryo::languages::node_table;

/// The reader the `parse` contract hands the `assigns` relation to, which reads its rows from a file's tree.
pub(crate) struct AssignsItem;

// The engine fills `assigns` from the walked file: each value an assignment or a declaration gives a name.
ryo::fills! {
    AssignsItem => languages::Assigns, walk(GoWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Assigns as languages::CatalogRelation>::Row;
        let _ = words;
        let tree = file.syntax;
        for wanted in [Kind::Node(Node::AssignStmt), Kind::Node(Node::ValueSpec)] {
            for &at in node_table::ids_of_kind(tree.table, kind_bits(wanted)) {
                for assigned in crate::domain::walked::definitions::assignments(&tree, at) {
                    rows.write(Row {
                        occurrence: walked_syntax::node_cell(&tree, at),
                        binding: walked_syntax::node_cell(&tree, assigned.binding),
                        value: walked_syntax::node_cell(&tree, assigned.value),
                        owner: crate::domain::walked::scopes::owner(&tree, at),
                        file: tree.file,
                        span: walked_syntax::span_of(&tree, assigned.value),
                    });
                }
            }
        }
    }
}
