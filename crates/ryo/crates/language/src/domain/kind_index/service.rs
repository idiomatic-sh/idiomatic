//! The `kind_index` service: free functions named by verb that group the positions of a table by kind, so the elements of a kind are found without a pass over the table.

use crate::domain::elements::service::{engine, engine_mut};
use crate::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::{Count, KindBits, Position};

/// Groups the positions of the table by kind, so the elements of a kind are found without a pass over the table.
/// Done once for a file, when its table is built.
pub fn index_kinds(elements: &mut Elements) {
    let table = engine_mut(elements);
    let mut starts = std::mem::take(&mut table.kind_start);
    let mut order = std::mem::take(&mut table.kind_order);
    let kinds = &elements.kind.as_ref().values;
    let count = kinds
        .iter()
        .copied()
        .max()
        .map_or(0, |highest| usize::from(highest.0) + 1);
    starts.clear();
    starts.resize(count + 1, Count(0));
    for each in kinds {
        if let Some(slot) = starts.get_mut(usize::from(each.0) + 1) {
            slot.0 += 1;
        }
    }
    let mut running = 0;
    for slot in &mut starts {
        running += slot.0;
        slot.0 = running;
    }
    order.clear();
    order.resize(kinds.len(), Position(0));
    // Each kind's start is its cursor while the positions are placed; it ends at the start of the next kind.
    for (position, each) in kinds.iter().enumerate() {
        if let Some(slot) = starts.get_mut(usize::from(each.0)) {
            if let Some(place) = order.get_mut(slot.0) {
                *place = Position(u32::try_from(position).unwrap_or(u32::MAX));
            }
            slot.0 += 1;
        }
    }
    // Each cursor now holds the start of the next kind: one step to the right, the first kind starting at zero, makes them starts.
    starts.rotate_right(1);
    if let Some(first) = starts.first_mut() {
        *first = Count(0);
    }
    let written = engine_mut(elements);
    written.kind_start = starts;
    written.kind_order = order;
}

/// The positions of the elements of the kind numbered `kind`, in order.
#[must_use]
pub fn of_kind(elements: &Elements, kind: KindBits) -> &[Position] {
    let at = usize::from(kind.0);
    let table = engine(elements);
    match (table.kind_start.get(at), table.kind_start.get(at + 1)) {
        (Some(from), Some(to)) => table.kind_order.get(from.0..to.0).unwrap_or(&[]),
        _ => &[],
    }
}
