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
pub(super) fn view(storage: &Arc<DerivedStorage>) -> Result<Arc<[Backing]>, OperatingError> {
    let header = size_of::<DerivedStorage>() + 2 * size_of::<usize>();
    let mut own = header;
    let mut output = Vec::new();
    match storage.as_ref() {
        DerivedStorage::RowIds { rows, .. } => output.push(arc_vec(rows, 1)),
        DerivedStorage::EqualityShared(index) => output.push(Backing {
            key: (11, Arc::as_ptr(index) as usize),
            retention_units: 0,
            bytes: crate::domain::stored_indexes::service::index_bytes(index)
                .ok_or(OperatingError::ShapeMismatch)?,
        }),
        DerivedStorage::Membership(MembershipData::Dense(values)) => {
            output.push(arc_vec(values, 10))
        }
        DerivedStorage::Postings(postings) => {
            let mut backing = arc_vec(postings, 12);
            backing.bytes += postings
                .iter()
                .map(|entry| entry.text.capacity())
                .sum::<usize>();
            output.push(backing);
            output.extend(postings.iter().map(|entry| arc_vec(&entry.rows, 1)));
        }
        DerivedStorage::Adjacency(graph) => {
            let mut bytes = storage_bytes(storage)?
                .checked_sub(header)
                .ok_or(OperatingError::ShapeMismatch)?;
            if let Some(rows) = &graph.identity.population.positions {
                let shared = arc_vec(rows, 1);
                bytes = bytes
                    .checked_sub(shared.bytes)
                    .ok_or(OperatingError::ShapeMismatch)?;
                output.push(shared);
            }
            output.push(Backing {
                key: (9, Arc::as_ptr(graph) as usize),
                retention_units: 0,
                bytes: bytes
                    + size_of::<crate::domain::stored_graph::types::CompactGraph>()
                    + 2 * size_of::<usize>(),
            });
        }
        _ => own = storage_bytes(storage)?,
    }
    output.push(Backing {
        key: (8, Arc::as_ptr(storage) as usize),
        retention_units: 0,
        bytes: own,
    });
    Ok(output.into())
}
