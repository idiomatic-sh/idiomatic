//! Immutable allocation descriptors, prepared only after a physical build.
use super::super::ledger::Backing;
use super::*;
fn arc_vec<T>(value: &Arc<Vec<T>>, tag: u8) -> Backing {
    Backing {
        key: (tag, Arc::as_ptr(value) as usize),
        retention_units: if tag == 1 { value.capacity() } else { 0 },
        bytes: size_of::<Vec<T>>() + 2 * size_of::<usize>() + value.capacity() * size_of::<T>(),
    }
}
pub(super) fn shared_index(
    index: &Arc<ryo_derived::domain::contract::types::KeyIndex>,
) -> Result<Backing, OperatingError> {
    Ok(Backing {
        key: (11, Arc::as_ptr(index) as usize),
        retention_units: 0,
        bytes: crate::domain::stored_indexes::service::index_bytes(index)
            .ok_or(OperatingError::ShapeMismatch)?,
    })
}
pub(super) fn view(storage: &Arc<DerivedStorage>) -> Result<Arc<[Backing]>, OperatingError> {
    DerivedStorage::backings(storage)
}
