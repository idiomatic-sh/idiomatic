//! The `mapping` relation of the language: every mapping of a `go.mod`, once: the file, which holds the `require` lines of its own,
//! and each `require` block. A go source is no keyed document and writes no mapping.

use crate::domain::extras::types::Extra;

use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

use crate::domain::file_view::types::GoWalk;
use crate::domain::walked::syntax as walked_syntax;
use ryo::languages::node_table;

/// The reader the `parse` contract hands the `mapping` relation to, which reads its rows from a file's tree.
pub(crate) struct MappingItem;

// The engine fills `mapping` from the walked file: a module file, and each block of requirements it holds.
ryo::fills! {
    MappingItem => languages::Mapping, walk(GoWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Mapping as languages::CatalogRelation>::Row;
        let _ = words;
        let tree = file.syntax;
        for wanted in [Kind::Extra(Extra::ModFile), Kind::Extra(Extra::ModLineBlock)] {
            for &at in node_table::ids_of_kind(tree.table, kind_bits(wanted)) {
                let Some(mapping) = crate::domain::walked::modules::mapping_of(&tree, at) else { continue };
                rows.write(Row {
                    id: walked_syntax::node_cell(&tree, at),
                    kind: mapping.kind,
                    ordinal: languages::Integer(0),
                    owner: walked_syntax::node_cell(&tree, mapping.owner),
                    file: tree.file,
                });
            }
        }
    }
}
