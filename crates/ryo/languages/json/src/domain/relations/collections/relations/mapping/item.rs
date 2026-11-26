//! The `mapping` relation of the language: every object, once. The object that is the whole document is the file's root mapping,
//! `standard`, anchored at the root; the value of a member is `inline`, and an element of an array a `sequence-member`.

use crate::domain::file_view::service::{modelled, node_cell};
use crate::domain::file_view::types::JsonWalk;
use crate::domain::walked_values::service as walked;
use ryo::languages::node_table;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The item the engine's fill of the relation is registered on.
pub(crate) struct MappingItem;

// The engine fills `mapping` from the walked file: every object, by the element it is known by.
ryo::fills! {
    MappingItem => languages::Mapping, walk(JsonWalk) |file, _words, rows| {
        /// One row of the relation.
        type Row = <languages::Mapping as languages::CatalogRelation>::Row;
        let (model, syntax) = modelled(file);
        for &id in node_table::ids_of_kind(syntax.table, kind_bits(Kind::Node(Node::Object))) {
            rows.write(Row {
                id: node_cell(&syntax, walked::mapping_id(&syntax, id)),
                kind: walked::mapping_kind(&syntax, id),
                ordinal: walked::mapping_ordinal(&syntax, model, id),
                owner: node_cell(&syntax, walked::holder(&syntax, id)),
                file: syntax.file,
            });
        }
    }
}
