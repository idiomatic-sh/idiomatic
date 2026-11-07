//! The `mapping` relation of the language: every mapping of a `go.mod`, once: the file, which holds the `require` lines of its own,
//! and each `require` block. A go source is no keyed document and writes no mapping.

use crate::domain::extras::types::Extra;
use crate::domain::modules::service::mapping_of;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `mapping` relation to, which reads its rows from a file's tree.
pub(crate) struct MappingItem;

ryo::reads! {
    MappingItem => languages::Mapping |tree, at, _symbols| {
        kinds: [Kind::Extra(Extra::ModFile), Kind::Extra(Extra::ModLineBlock)] as kind_bits,
        given mapping = mapping_of(tree, at)?;
        row {
            id: node(at),
            kind: value(mapping.kind),
            ordinal: value(languages::Integer(0)),
            owner: node(mapping.owner),
            file: file,
        }
    }
}
