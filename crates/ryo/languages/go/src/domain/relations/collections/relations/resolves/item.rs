//! The `resolves` relation of the language: each identifier written as a type, with the `type` declaration of the file or the block
//! that binds it where it is written. A name qualified by a package (`pkg.Type`) and a type of another file of the package are not
//! resolved.


use crate::domain::nodes::types::Node;

use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

use crate::domain::file_view::types::{GoWalk, Words};
use crate::domain::walked::syntax as walked_syntax;
use ryo::languages::node_table;

/// The reader the `parse` contract hands the `resolves` relation to, which reads its rows from a file's tree.
pub(crate) struct ResolvesItem;

// The engine fills `resolves` from the walked file: each identifier that names a type declared in an enclosing scope.
ryo::fills! {
    ResolvesItem => languages::Resolves, walk(GoWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Resolves as languages::CatalogRelation>::Row;
        let tree = file.syntax;
        let words = Words(std::cell::RefCell::new(words));
        for &at in node_table::ids_of_kind(tree.table, kind_bits(Kind::Node(Node::Ident))) {
            let Some(typed) = crate::domain::walked::specs::written_as_type(&tree, at) else { continue };
            let Some(name) = crate::domain::walked::names::name_of(&tree, &words, typed) else { continue };
            let Some(definition) = crate::domain::walked::specs::binding_of(&tree, &words, at, name) else { continue };
            rows.write(Row {
                occurrence: walked_syntax::node_cell(&tree, at),
                owner: crate::domain::walked::specs::holder(&tree, at),
                name,
                definition: walked_syntax::node_cell(&tree, definition),
                file: tree.file,
                span: walked_syntax::span_of(&tree, at),
            });
        }
    }
}
