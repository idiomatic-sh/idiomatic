//! File requirements shared by the provider visit and executor.
use crate::languages as l;
use ryo_relations::domain::labels::types::NameRef;
use ryo_relations::domain::relations::service as catalog;

pub mod packet;

pub(crate) mod pairs;


/// SDK packets have node-addressable columns or file-wide rows.
pub(crate) fn fact_family(name: &str) -> super::pair_summary::cost::Family {
    use super::pair_summary::cost::Family;
    match catalog::relation_named(NameRef(name)) {
        Some(rel) if !catalog::kinds(rel).contains(&l::CellKind::Node) => Family::Sets,
        _ => Family::Fact,
    }
}
