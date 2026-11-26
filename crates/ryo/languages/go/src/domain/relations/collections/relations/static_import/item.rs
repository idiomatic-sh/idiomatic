//! The `static_import` relation of the language: every import, by the path it imports. Its binding is the name it gives the package;
//! an import without one binds the name the package declares, which the file does not say, and `import . "p"` binds every exported
//! name of `p`: each has the unknown value of that.


use crate::domain::nodes::types::Node;

use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

use crate::domain::file_view::types::{GoWalk, Words};
use crate::domain::walked::syntax as walked_syntax;
use ryo::languages::node_table;

/// The reader the `parse` contract hands the `static_import` relation to, which reads its rows from a file's tree.
pub(crate) struct StaticImportItem;

// The engine fills `static_import` from the walked file: every import spec and the package it names.
ryo::fills! {
    StaticImportItem => languages::StaticImport, walk(GoWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::StaticImport as languages::CatalogRelation>::Row;
        let tree = file.syntax;
        let words = Words(std::cell::RefCell::new(words));
        for &at in node_table::ids_of_kind(tree.table, kind_bits(Kind::Node(Node::ImportSpec))) {
            let Some(target) = crate::domain::walked::names::import_target(&tree, &words, at) else { continue };
            rows.write(Row {
                occurrence: walked_syntax::node_cell(&tree, at),
                owner: crate::domain::walked::scopes::owner(&tree, at),
                binding: crate::domain::walked::names::import_binding(&tree, &words, at),
                target,
                file: tree.file,
                span: walked_syntax::span_of(&tree, at),
            });
        }
    }
}
