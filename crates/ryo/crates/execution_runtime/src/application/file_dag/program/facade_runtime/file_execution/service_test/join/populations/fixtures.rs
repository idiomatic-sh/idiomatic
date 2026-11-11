//! The support of the `populations` cases.
use super::super::cases::fixtures::{node, stretch};
use super::super::*;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{FileId, Relation, Value as Cell},
};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::Table;

pub(super) fn functions(count: usize, dense: bool) -> Table {
    let mut table = Table {
        kinds: kinds(Relation::Function).to_vec(),
        ..Table::default()
    };
    for place in 0..count {
        push_row(
            &mut table,
            &[
                node(if dense { 1 } else { (place % 8 + 1) as u32 }),
                Cell::Symbol(Symbol(1)),
                Cell::File(FileId((place % 2) as u32)),
                stretch(place as u32, place as u32 + 1),
            ],
        );
    }
    table
}
