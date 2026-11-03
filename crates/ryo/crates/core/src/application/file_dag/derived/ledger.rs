//! Accounting for physical payloads held by derived resource handles.
//! Admission policy, cache selection and scheduling stay with the caller.
use crate::domain::operating::errors::OperatingError;
use std::{cell::RefCell, collections::HashMap, mem::size_of, rc::Rc};

#[cfg(test)]
#[path = "ledger_test.rs"]
mod tests;

type Key = (u8, usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) struct Backing {
    pub key: Key,
    pub bytes: usize,
    /// Immutable row-position capacity when this backing is optionally retained.
    pub retention_units: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LeaseClass {
    Required,
    /// Derived-view retention, subject to the view byte and position caps.
    Optional,
    /// DAG node retention, subject to its own byte cap.
    NodeOptional,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Snapshot {
    pub required_bytes: usize,
    /// Physical bytes with either optional role, counted once per backing.
    pub optional_bytes: usize,
    pub view_optional_bytes: usize,
    pub node_optional_bytes: usize,
    pub optional_units: usize,
    pub overlap_bytes: usize,
    pub unique_bytes: usize,
    pub metadata_bytes: usize,
    pub peak_required_bytes: usize,
    pub peak_optional_bytes: usize,
    pub peak_view_optional_bytes: usize,
    pub peak_node_optional_bytes: usize,
    pub peak_optional_units: usize,
    pub peak_overlap_bytes: usize,
    pub peak_unique_bytes: usize,
    pub peak_metadata_bytes: usize,
}

#[derive(Debug)]
struct Entry {
    bytes: usize,
    retention_units: usize,
    required: usize,
    optional: usize,
    node_optional: usize,
}

#[derive(Debug)]
pub(super) struct Ledger {
    entries: HashMap<Key, Entry>,
    required_bytes: usize,
    optional_bytes: usize,
    view_optional_bytes: usize,
    node_optional_bytes: usize,
    optional_units: usize,
    overlap_bytes: usize,
    unique_bytes: usize,
    lease_metadata: usize,
    peak_required_bytes: usize,
    peak_optional_bytes: usize,
    peak_view_optional_bytes: usize,
    peak_node_optional_bytes: usize,
    peak_optional_units: usize,
    peak_overlap_bytes: usize,
    peak_unique_bytes: usize,
    peak_metadata_bytes: usize,
}

impl Default for Ledger {
    fn default() -> Self {
        let mut ledger = Self {
            entries: HashMap::new(),
            required_bytes: 0,
            optional_bytes: 0,
            view_optional_bytes: 0,
            node_optional_bytes: 0,
            optional_units: 0,
            overlap_bytes: 0,
            unique_bytes: 0,
            lease_metadata: 0,
            peak_required_bytes: 0,
            peak_optional_bytes: 0,
            peak_view_optional_bytes: 0,
            peak_node_optional_bytes: 0,
            peak_optional_units: 0,
            peak_overlap_bytes: 0,
            peak_unique_bytes: 0,
            peak_metadata_bytes: 0,
        };
        ledger.note_peaks();
        ledger
    }
}

impl Ledger {
    /// Bytes newly retained by the view optional class. A required-only or
    /// node-only backing is charged because a view lease may outlive it.
    pub(super) fn incremental_optional_bytes(
        &self,
        backings: &[Backing],
    ) -> Result<usize, OperatingError> {
        let mut additional = 0usize;
        for (key, bytes, units) in normalized(backings)? {
            match self.entries.get(&key) {
                Some(entry) if entry.bytes != bytes || entry.retention_units != units => {
                    return Err(OperatingError::ShapeMismatch);
                }
                Some(entry) if entry.optional == 0 => {
                    additional = additional
                        .checked_add(bytes)
                        .ok_or(OperatingError::ShapeMismatch)?
                }
                Some(_) => {}
                None => {
                    additional = additional
                        .checked_add(bytes)
                        .ok_or(OperatingError::ShapeMismatch)?
                }
            }
        }
        Ok(additional)
    }

    /// Bytes newly retained by the node optional class. View-only aliases
    /// still consume this separate node allowance.
    pub(super) fn incremental_node_optional_bytes(
        &self,
        backings: &[Backing],
    ) -> Result<usize, OperatingError> {
        let mut additional = 0usize;
        for (key, bytes, units) in normalized(backings)? {
            match self.entries.get(&key) {
                Some(entry) if entry.bytes != bytes || entry.retention_units != units => {
                    return Err(OperatingError::ShapeMismatch);
                }
                Some(entry) if entry.node_optional != 0 => {}
                _ => {
                    additional = additional
                        .checked_add(bytes)
                        .ok_or(OperatingError::ShapeMismatch)?;
                }
            }
        }
        Ok(additional)
    }

    /// Position capacity newly retained by the optional class, independent of
    /// the byte limit. Required-only aliases still consume this allowance.
    pub(super) fn incremental_optional_units(
        &self,
        backings: &[Backing],
    ) -> Result<usize, OperatingError> {
        let mut additional = 0usize;
        for (key, bytes, units) in normalized(backings)? {
            match self.entries.get(&key) {
                Some(entry) if entry.bytes != bytes || entry.retention_units != units => {
                    return Err(OperatingError::ShapeMismatch);
                }
                Some(entry) if entry.optional != 0 => {}
                _ => {
                    additional = additional
                        .checked_add(units)
                        .ok_or(OperatingError::ShapeMismatch)?;
                }
            }
        }
        Ok(additional)
    }

    /// Register one handle. The caller may share it through an outer `Arc`;
    /// cloning that `Arc` does not create another ledger lease.
    pub(super) fn acquire(
        ledger: &Rc<RefCell<Self>>,
        backings: &[Backing],
        class: LeaseClass,
    ) -> Result<ResourceLease, OperatingError> {
        let unique = normalized(backings)?;
        let mut keys = Vec::with_capacity(unique.len());
        let mut state = ledger.borrow_mut();
        let mut next_required = state.required_bytes;
        let mut next_optional = state.optional_bytes;
        let mut next_view_optional = state.view_optional_bytes;
        let mut next_node_optional = state.node_optional_bytes;
        let mut next_optional_units = state.optional_units;
        let mut next_overlap = state.overlap_bytes;
        let mut next_unique = state.unique_bytes;
        for (key, bytes, units) in &unique {
            let old = state.entries.get(key);
            if old.is_some_and(|entry| entry.bytes != *bytes || entry.retention_units != *units) {
                return Err(OperatingError::ShapeMismatch);
            }
            let required = old.map_or(0, |entry| entry.required);
            let optional = old.map_or(0, |entry| entry.optional);
            let node_optional = old.map_or(0, |entry| entry.node_optional);
            match class {
                LeaseClass::Required => {
                    required
                        .checked_add(1)
                        .ok_or(OperatingError::ShapeMismatch)?;
                    if required == 0 {
                        next_required = next_required
                            .checked_add(*bytes)
                            .ok_or(OperatingError::ShapeMismatch)?;
                        if optional != 0 || node_optional != 0 {
                            next_overlap = next_overlap
                                .checked_add(*bytes)
                                .ok_or(OperatingError::ShapeMismatch)?;
                        }
                    }
                }
                LeaseClass::Optional => {
                    optional
                        .checked_add(1)
                        .ok_or(OperatingError::ShapeMismatch)?;
                    if optional == 0 {
                        next_view_optional = next_view_optional
                            .checked_add(*bytes)
                            .ok_or(OperatingError::ShapeMismatch)?;
                        next_optional_units = next_optional_units
                            .checked_add(*units)
                            .ok_or(OperatingError::ShapeMismatch)?;
                        if node_optional == 0 {
                            next_optional = next_optional
                                .checked_add(*bytes)
                                .ok_or(OperatingError::ShapeMismatch)?;
                        }
                        if required != 0 && node_optional == 0 {
                            next_overlap = next_overlap
                                .checked_add(*bytes)
                                .ok_or(OperatingError::ShapeMismatch)?;
                        }
                    }
                }
                LeaseClass::NodeOptional => {
                    node_optional
                        .checked_add(1)
                        .ok_or(OperatingError::ShapeMismatch)?;
                    if node_optional == 0 {
                        next_node_optional = next_node_optional
                            .checked_add(*bytes)
                            .ok_or(OperatingError::ShapeMismatch)?;
                        if optional == 0 {
                            next_optional = next_optional
                                .checked_add(*bytes)
                                .ok_or(OperatingError::ShapeMismatch)?;
                        }
                        if required != 0 && optional == 0 {
                            next_overlap = next_overlap
                                .checked_add(*bytes)
                                .ok_or(OperatingError::ShapeMismatch)?;
                        }
                    }
                }
            }
            if old.is_none() {
                next_unique = next_unique
                    .checked_add(*bytes)
                    .ok_or(OperatingError::ShapeMismatch)?;
            }
            keys.push(*key);
        }
        let lease_bytes = lease_bytes(keys.capacity())?;
        let next_lease_metadata = state
            .lease_metadata
            .checked_add(lease_bytes)
            .ok_or(OperatingError::ShapeMismatch)?;
        for (key, bytes, units) in unique {
            let entry = state.entries.entry(key).or_insert(Entry {
                bytes,
                retention_units: units,
                required: 0,
                optional: 0,
                node_optional: 0,
            });
            match class {
                LeaseClass::Required => entry.required += 1,
                LeaseClass::Optional => entry.optional += 1,
                LeaseClass::NodeOptional => entry.node_optional += 1,
            }
        }
        state.required_bytes = next_required;
        state.optional_bytes = next_optional;
        state.view_optional_bytes = next_view_optional;
        state.node_optional_bytes = next_node_optional;
        state.optional_units = next_optional_units;
        state.overlap_bytes = next_overlap;
        state.unique_bytes = next_unique;
        state.lease_metadata = next_lease_metadata;
        state.note_peaks();
        Ok(ResourceLease {
            ledger: Rc::clone(ledger),
            keys,
            class,
            metadata_bytes: lease_bytes,
        })
    }

    pub(super) fn snapshot(&self) -> Snapshot {
        Snapshot {
            required_bytes: self.required_bytes,
            optional_bytes: self.optional_bytes,
            view_optional_bytes: self.view_optional_bytes,
            node_optional_bytes: self.node_optional_bytes,
            optional_units: self.optional_units,
            overlap_bytes: self.overlap_bytes,
            unique_bytes: self.unique_bytes,
            metadata_bytes: self.metadata_bytes(),
            peak_required_bytes: self.peak_required_bytes,
            peak_optional_bytes: self.peak_optional_bytes,
            peak_view_optional_bytes: self.peak_view_optional_bytes,
            peak_node_optional_bytes: self.peak_node_optional_bytes,
            peak_optional_units: self.peak_optional_units,
            peak_overlap_bytes: self.peak_overlap_bytes,
            peak_unique_bytes: self.peak_unique_bytes,
            peak_metadata_bytes: self.peak_metadata_bytes,
        }
    }

    fn metadata_bytes(&self) -> usize {
        // A conservative bucket estimate plus the actual reserved lease key
        // capacities. No iteration over live resources is needed for a snapshot.
        size_of::<Self>()
            .saturating_add(
                self.entries
                    .capacity()
                    .saturating_mul(size_of::<(Key, Entry)>() + 1),
            )
            .saturating_add(self.lease_metadata)
    }

    fn note_peaks(&mut self) {
        self.peak_required_bytes = self.peak_required_bytes.max(self.required_bytes);
        self.peak_optional_bytes = self.peak_optional_bytes.max(self.optional_bytes);
        self.peak_view_optional_bytes = self.peak_view_optional_bytes.max(self.view_optional_bytes);
        self.peak_node_optional_bytes = self.peak_node_optional_bytes.max(self.node_optional_bytes);
        self.peak_optional_units = self.peak_optional_units.max(self.optional_units);
        self.peak_overlap_bytes = self.peak_overlap_bytes.max(self.overlap_bytes);
        self.peak_unique_bytes = self.peak_unique_bytes.max(self.unique_bytes);
        self.peak_metadata_bytes = self.peak_metadata_bytes.max(self.metadata_bytes());
    }
}

/// One physical handle, shared by consumers through an outer `Arc`.
#[derive(Debug)]
pub(in crate::application::file_dag) struct ResourceLease {
    ledger: Rc<RefCell<Ledger>>,
    keys: Vec<Key>,
    class: LeaseClass,
    metadata_bytes: usize,
}

impl Drop for ResourceLease {
    fn drop(&mut self) {
        let mut state = self.ledger.borrow_mut();
        for key in &self.keys {
            let (bytes, units, class_empty, required_live, optional_live, remove) = {
                let entry = state
                    .entries
                    .get_mut(key)
                    .expect("live backing for resource lease");
                match self.class {
                    LeaseClass::Required => entry.required -= 1,
                    LeaseClass::Optional => entry.optional -= 1,
                    LeaseClass::NodeOptional => entry.node_optional -= 1,
                }
                (
                    entry.bytes,
                    entry.retention_units,
                    match self.class {
                        LeaseClass::Required => entry.required == 0,
                        LeaseClass::Optional => entry.optional == 0,
                        LeaseClass::NodeOptional => entry.node_optional == 0,
                    },
                    entry.required != 0,
                    entry.optional != 0 || entry.node_optional != 0,
                    entry.required == 0 && entry.optional == 0 && entry.node_optional == 0,
                )
            };
            if class_empty {
                match self.class {
                    LeaseClass::Required => state.required_bytes -= bytes,
                    LeaseClass::Optional => {
                        state.view_optional_bytes -= bytes;
                        state.optional_units -= units;
                    }
                    LeaseClass::NodeOptional => state.node_optional_bytes -= bytes,
                }
                if matches!(self.class, LeaseClass::Optional | LeaseClass::NodeOptional)
                    && !optional_live
                {
                    state.optional_bytes -= bytes;
                }
                if (matches!(self.class, LeaseClass::Required) && optional_live)
                    || (matches!(self.class, LeaseClass::Optional | LeaseClass::NodeOptional)
                        && required_live
                        && !optional_live)
                {
                    state.overlap_bytes -= bytes;
                }
            }
            if remove {
                state.unique_bytes -= bytes;
                state.entries.remove(key);
            }
        }
        state.lease_metadata -= self.metadata_bytes;
    }
}

fn normalized(backings: &[Backing]) -> Result<Vec<(Key, usize, usize)>, OperatingError> {
    let mut seen = HashMap::with_capacity(backings.len());
    for backing in backings {
        if seen
            .insert(backing.key, (backing.bytes, backing.retention_units))
            .is_some_and(|prior| prior != (backing.bytes, backing.retention_units))
        {
            return Err(OperatingError::ShapeMismatch);
        }
    }
    Ok(seen
        .into_iter()
        .map(|(key, (bytes, units))| (key, bytes, units))
        .collect())
}

fn lease_bytes(key_capacity: usize) -> Result<usize, OperatingError> {
    size_of::<ResourceLease>()
        .checked_add(2 * size_of::<usize>()) // outer Arc allocation header
        .and_then(|base| {
            key_capacity
                .checked_mul(size_of::<Key>())
                .and_then(|keys| base.checked_add(keys))
        })
        .ok_or(OperatingError::ShapeMismatch)
}
