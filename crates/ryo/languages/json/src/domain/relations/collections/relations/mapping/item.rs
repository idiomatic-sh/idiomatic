//! The `mapping` relation of the language: every object, once. The object that is the whole document is the file's root mapping,
//! `standard`, anchored at the root; the value of a member is `inline`, and an element of an array a `sequence-member`.

use crate::domain::file_view::service::{modelled, node_cell};
use crate::domain::file_view::types::JsonWalk;
use crate::domain::walked_values::service as walked;
use ryo::languages::node_table;
use crate::domain::model::service::mapping_ordinal;
use crate::domain::model::types::Model;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{holder, kind_bits, mapping_id, mapping_kind};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `mapping` relation to, which reads its rows from a file's tree.
pub(crate) struct MappingItem;

ryo::reads! {
    MappingItem => languages::Mapping, model(Model) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Object)] as kind_bits,
        row {
            id: node(mapping_id(tree, at)),
            kind: value(mapping_kind(tree, at)),
            ordinal: value(mapping_ordinal(tree, model, at)),
            owner: node(holder(tree, at)),
            file: file,
        }
    }
}

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
