//! The `mapping` relation of the language: every object, once. The object that is the whole document is the file's root mapping,
//! `standard`, anchored at the root; the value of a member is `inline`, and an element of an array a `sequence-member`.

use crate::domain::model::service::mapping_ordinal;
use crate::domain::model::types::Model;
use crate::domain::nodes::types::JsonNode;
use crate::domain::syntax::service::{holder, kind_bits, mapping_id, mapping_kind};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `mapping` relation to, which reads its rows from a file's tree.
pub(crate) struct MappingItem;

ryo::reads! {
    MappingItem => languages::Mapping, model(Model) |tree, model, at, symbols| {
        kinds: [Kind::Node(JsonNode::Object)] as kind_bits,
        row {
            id: node(mapping_id(tree, at)),
            kind: value(mapping_kind(tree, at)),
            ordinal: value(mapping_ordinal(tree, model, at)),
            owner: node(holder(tree, at)),
            file: file,
        }
    }
}
