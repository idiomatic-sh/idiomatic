//! Capacity accounting must include reserved room, not merely logical lengths.

use super::{arc_bytes, vector};

#[test]
fn vector_accounting_charges_unused_capacity_and_arc_header() {
    let mut values = Vec::<u32>::with_capacity(19);
    values.push(7);
    assert_eq!(vector(&values).0, values.capacity() * size_of::<u32>());
    assert!(arc_bytes::<Vec<u32>>().0 >= size_of::<Vec<u32>>() + 2 * size_of::<usize>());
}

#[test]
fn compact_table_accounting_charges_header_and_reserved_vectors() {
    use ryo_values::domain::stored_facts::types::{KeptWords, PackedWords};
    use ryo_values::domain::table_words::types;
    let admitted = ryo_values::domain::compact_table::service::admit_compact(
        types::CompactBacking(std::sync::Arc::new(KeptWords(PackedWords::default()))),
        &mut Vec::new(),
        types::DecodeMode::Complete,
    )
    .unwrap();
    let table = admitted.table;
    assert!(
        super::compact_table(&table).0
            >= size_of::<types::CompactTable>()
                + table.row_offsets.capacity() * size_of::<types::FrameOffset>()
    );
}
