//! Capacity accounting must include reserved room, not merely logical lengths.

use super::{arc_bytes, vector};

#[test]
fn vector_accounting_charges_unused_capacity_and_arc_header() {
    let mut values = Vec::<u32>::with_capacity(19);
    values.push(7);
    assert_eq!(vector(&values), values.capacity() * size_of::<u32>());
    assert!(arc_bytes::<Vec<u32>>() >= size_of::<Vec<u32>>() + 2 * size_of::<usize>());
}
