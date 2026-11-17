// ignore: idiomatic.arch/file-length - the concept spec places one service.rs per concept, and the steps of the fold (links, validate, reconcile, descends, snapshot) read one operation graph; perf's service imports this concept's types, so moving a step there is an import cycle between the two concepts
//! The `operations` service: the fold of an operation graph, with no IO and no ports.
//!
//! Descendant decisions supersede ancestor ones; a concurrent removal beats an acceptance, so uncertainty fails
//! closed; concurrent acceptances keep every reason and the stricter count.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::constants::COMPACT_AT;
use super::errors::OperationsError;
use super::types::{
    Count, Decision, Descent, Folded, Lineage, Maximal, Operation, OperationId, PerfDecision,
    PerfKey, PerfMaximal, PerfRecord, Record, Removal, Shape,
};
use crate::domain::violations::types::{Id, Standing};

/// What is already known of which operation descends from which.
type Reach = HashMap<Lineage, Descent>;

/// How many parents of each operation are in the graph, and which operations name each as a parent.
type Links<'graph> = (
    BTreeMap<&'graph OperationId, usize>,
    BTreeMap<&'graph OperationId, Vec<&'graph OperationId>>,
);

/// How many parents of each operation are in the graph, and which operations name each as a parent.
///
/// # Errors
///
/// `Invalid` or `MissingParent` when a record or a parent link is invalid.
fn links<'graph>(
    operations: &'graph BTreeMap<OperationId, Operation>,
    covered: &BTreeSet<&OperationId>,
) -> Result<Links<'graph>, OperationsError> {
    let mut remaining: BTreeMap<&OperationId, usize> = BTreeMap::new();
    let mut children: BTreeMap<&OperationId, Vec<&OperationId>> = BTreeMap::new();
    for (id, operation) in operations {
        validate(id, operation)?;
        let mut present = 0_usize;
        for parent in &operation.parents {
            let held = operations.contains_key(parent);
            if !held && !covered.contains(parent) {
                return Err(OperationsError::MissingParent {
                    operation: id.0.to_string(),
                    parent: parent.0.to_string(),
                });
            }
            if held {
                present += 1;
                children.entry(parent).or_default().push(id);
            }
        }
        let _: Option<usize> = remaining.insert(id, present);
    }
    Ok((remaining, children))
}

/// Folds a complete operation graph.
///
/// # Errors
///
/// `Invalid`, `Twice`, `MissingParent` or `Cycle` when a record, a parent link or the graph is invalid.
pub fn fold(operations: &BTreeMap<OperationId, Operation>) -> Result<Folded, OperationsError> {
    // A parent a snapshot covers was replaced by it: the operation descends from what the snapshot holds, and is
    // concurrent with the snapshot, so the link is no edge of the graph.
    let covered: BTreeSet<&OperationId> = operations
        .values()
        .flat_map(|operation| &operation.covers)
        .collect();
    let (mut remaining, children) = links(operations, &covered)?;
    let parents: BTreeSet<&OperationId> = operations
        .values()
        .flat_map(|operation| &operation.parents)
        .collect();
    let mut ready: BTreeSet<&OperationId> = remaining
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(id, _)| *id)
        .collect();
    let mut decisions: BTreeMap<Id, Vec<Maximal>> = BTreeMap::new();
    let mut perf: BTreeMap<PerfKey, Vec<PerfMaximal>> = BTreeMap::new();
    let mut tombstones: BTreeMap<Id, Removal> = BTreeMap::new();
    let mut reach = Reach::new();
    let mut visited = 0_usize;
    while let Some(id) = ready.pop_first() {
        visited += 1;
        let operation = &operations[id];
        let made = operation
            .accepted
            .iter()
            .map(|record| (record.id.clone(), Decision::Accept(record.clone())))
            .chain(
                operation
                    .removed
                    .iter()
                    .map(|removal| (removal.id.clone(), Decision::Remove)),
            );
        for removal in &operation.removed {
            drop(tombstones.insert(removal.id.clone(), removal.clone()));
        }
        for (record, decision) in made {
            let maximal = decisions.entry(record).or_default();
            maximal.retain(|prior| {
                descends(id, &prior.operation, operations, &mut reach) == Descent::Apart
            });
            maximal.push(Maximal {
                operation: id.clone(),
                decision,
            });
        }
        let set = operation
            .perf
            .iter()
            .map(|record| (record.key.clone(), PerfDecision::Set(record.clone())));
        let unset = operation
            .perf_removed
            .iter()
            .map(|key| (key.clone(), PerfDecision::Unset));
        for (key, decision) in set.chain(unset) {
            let maximal = perf.entry(key).or_default();
            maximal.retain(|prior| {
                descends(id, &prior.operation, operations, &mut reach) == Descent::Apart
            });
            maximal.push(PerfMaximal {
                operation: id.clone(),
                decision,
            });
        }
        for child in children.get(id).into_iter().flatten() {
            if let Some(count) = remaining.get_mut(child) {
                *count -= 1;
                if *count == 0 {
                    // A child is ready once, when its last parent is folded, so it is never already there.
                    ready.extend([child]);
                }
            }
        }
    }
    if visited != operations.len() {
        return Err(OperationsError::Cycle);
    }
    let mut folded = reconcile(decisions, &tombstones);
    folded.perf_removals = perf
        .iter()
        .filter(|(_, maximal)| {
            maximal
                .iter()
                .any(|kept| matches!(kept.decision, PerfDecision::Unset))
        })
        .map(|(key, _)| key.clone())
        .collect();
    folded.perf = reconcile_perf(perf);
    folded.heads = operations
        .keys()
        .filter(|id| !parents.contains(*id))
        .cloned()
        .collect();
    Ok(folded)
}

/// The perf records each key's maximal decisions come to: a concurrent removal drops the key; concurrent settings keep
/// the strictest, the lowest best, so uncertainty fails closed.
fn reconcile_perf(decisions: BTreeMap<PerfKey, Vec<PerfMaximal>>) -> Vec<PerfRecord> {
    decisions
        .into_values()
        .filter(|maximal| {
            maximal
                .iter()
                .all(|kept| matches!(kept.decision, PerfDecision::Set(_)))
        })
        .filter_map(|maximal| {
            maximal
                .into_iter()
                .filter_map(|kept| match kept.decision {
                    PerfDecision::Set(record) => Some((kept.operation, record)),
                    PerfDecision::Unset => None,
                })
                .min_by(|(left_id, left), (right_id, right)| {
                    left.best
                        .0
                        .total_cmp(&right.best.0)
                        .then_with(|| left_id.cmp(right_id))
                })
                .map(|(_, record)| record)
        })
        .collect()
}

/// Refuses an operation with a record of count zero, with no reason, or decided twice.
fn validate(id: &OperationId, operation: &Operation) -> Result<(), OperationsError> {
    let mut decided = BTreeSet::new();
    for record in &operation.accepted {
        // A reported record is accepted for a reason; a suppressed one's is in the source.
        let unreasoned = match record.key.standing {
            Standing::Reported => record.reasons.is_empty(),
            Standing::Suppressed => !record.reasons.is_empty(),
        };
        if record.count == Count(0)
            || unreasoned
            || record.reasons.iter().any(|reason| reason.0.is_empty())
        {
            return Err(OperationsError::Invalid {
                operation: id.0.to_string(),
                record: record.id.0.to_string(),
            });
        }
    }
    let ids = operation
        .accepted
        .iter()
        .map(|record| &record.id)
        .chain(operation.removed.iter().map(|removal| &removal.id));
    for record in ids {
        if !decided.insert(record) {
            return Err(OperationsError::Twice {
                operation: id.0.to_string(),
                record: record.0.to_string(),
            });
        }
    }
    let mut set = BTreeSet::new();
    let keys = operation
        .perf
        .iter()
        .map(|record| &record.key)
        .chain(&operation.perf_removed);
    for key in keys {
        if !set.insert(key) {
            return Err(OperationsError::Twice {
                operation: id.0.to_string(),
                record: format!("{}/{}/{}", key.tool.0, key.benchmark.0, key.measure.0),
            });
        }
    }
    for record in &operation.perf {
        let unusable = !record.best.0.is_finite() || record.best.0 < 0.0_f64;
        if unusable
            || record.reasons.is_empty()
            || record.reasons.iter().any(|reason| reason.0.is_empty())
        {
            return Err(OperationsError::Invalid {
                operation: id.0.to_string(),
                record: format!(
                    "{}/{}/{}",
                    record.key.tool.0, record.key.benchmark.0, record.key.measure.0
                ),
            });
        }
    }
    Ok(())
}

/// The records each record's maximal decisions accept, with the operations that accepted them, and the removals that
/// stand: a concurrent removal drops a record; concurrent acceptances keep every reason and the stricter count.
fn reconcile(decisions: BTreeMap<Id, Vec<Maximal>>, tombstones: &BTreeMap<Id, Removal>) -> Folded {
    let mut records = Vec::new();
    let mut removals = Vec::new();
    let mut origins = BTreeMap::new();
    for (id, maximal) in decisions {
        if maximal.iter().any(|kept| kept.decision == Decision::Remove) {
            removals.extend(tombstones.get(&id).cloned());
            continue;
        }
        let mut accepted: Vec<Maximal> = maximal;
        accepted.sort_by(|left, right| left.operation.cmp(&right.operation));
        let records_of = accepted.iter().filter_map(|kept| match &kept.decision {
            Decision::Accept(record) => Some(record),
            Decision::Remove => None,
        });
        let mut merged: Option<Record> = None;
        let mut reasons = BTreeSet::new();
        for candidate in records_of {
            reasons.extend(candidate.reasons.iter().cloned());
            let record = merged.get_or_insert_with(|| candidate.clone());
            record.count = record.count.min(candidate.count);
        }
        let Some(mut record) = merged else {
            continue;
        };
        record.reasons = reasons.into_iter().collect();
        // Each id is folded once, so no origins are replaced.
        origins.extend([(
            id,
            accepted.into_iter().map(|kept| kept.operation).collect(),
        )]);
        records.push(record);
    }
    records.sort_by(|left, right| left.id.cmp(&right.id));
    Folded {
        records,
        removals,
        heads: Vec::new(),
        origins,
        perf: Vec::new(),
        perf_removals: Vec::new(),
    }
}

/// Whether `operation` descends from `ancestor`, itself included.
fn descends(
    operation: &OperationId,
    ancestor: &OperationId,
    operations: &BTreeMap<OperationId, Operation>,
    reach: &mut Reach,
) -> Descent {
    let key = Lineage {
        operation: operation.clone(),
        ancestor: ancestor.clone(),
    };
    if let Some(known) = reach.get(&key) {
        return *known;
    }
    let mut pending = vec![operation];
    let mut seen = BTreeSet::new();
    let mut found = Descent::Apart;
    while let Some(current) = pending.pop() {
        if current == ancestor {
            found = Descent::Descends;
            break;
        }
        if seen.insert(current) {
            pending.extend(
                operations
                    .get(current)
                    .into_iter()
                    .flat_map(|held| &held.parents),
            );
        }
    }
    reach.extend([(key, found)]);
    found
}

/// Whether `folded` is already one snapshot: one head that has no parent and accepted every record it holds, of which there is one
/// at least, since a lock with no record cannot tell a snapshot from a chain of operations that drained it; a head with a parent
/// is the end of a chain, however many of the records it accepted last.
#[must_use]
pub fn shape(operations: &BTreeMap<OperationId, Operation>, folded: &Folded) -> Shape {
    let single = match folded.heads.as_slice() {
        [] => true,
        [head] => {
            operations
                .get(head)
                .is_some_and(|held| held.parents.is_empty())
                && !folded.records.is_empty()
                && folded.origins.len() == folded.records.len()
                && folded
                    .origins
                    .values()
                    .all(|origins| origins.as_slice() == std::slice::from_ref(head))
        }
        _ => false,
    };
    if single {
        Shape::Compacted
    } else {
        Shape::Open
    }
}

/// Whether the lock is compacted enough: `Compacted` while it holds fewer than `COMPACT_AT` operations, so a snapshot is
/// appended once every `COMPACT_AT` operations, never with every write. A snapshot replaces every operation it covers, whose
/// files go, and has no parent: the operations the lock holds are the snapshot and those since, whatever the records they
/// accepted, so a record that changes with every operation compacts as any other.
#[must_use]
pub fn settled(operations: &BTreeMap<OperationId, Operation>) -> Shape {
    if operations.len() < COMPACT_AT {
        Shape::Compacted
    } else {
        Shape::Open
    }
}

/// The snapshot of `operations`, folded to `folded`, carried to `records`: one operation with no parent that covers
/// every operation of the lock and every operation those covered, accepts every record, and removes the ones `records`
/// no longer holds together with every removal that still stands; it folds to exactly `records`, becomes every
/// record's origin, and replaces the operations it covers. The removals stand so an operation concurrent with the
/// snapshot, one whose parent it covers, meets the same removal it would have met uncompacted.
#[must_use]
pub fn snapshot(
    operations: &BTreeMap<OperationId, Operation>,
    folded: &Folded,
    records: &[Record],
) -> Operation {
    let kept: BTreeSet<&Id> = records.iter().map(|record| &record.id).collect();
    let covers: BTreeSet<OperationId> = operations
        .iter()
        .flat_map(|(id, operation)| std::iter::once(id).chain(&operation.covers))
        .cloned()
        .collect();
    let dropped = folded
        .records
        .iter()
        .filter(|record| !kept.contains(&record.id))
        .map(|record| Removal {
            id: record.id.clone(),
            key: record.key.clone(),
        });
    let standing = folded
        .removals
        .iter()
        .filter(|removal| !kept.contains(&removal.id))
        .cloned();
    Operation {
        parents: Vec::new(),
        covers: covers.into_iter().collect(),
        accepted: records.to_vec(),
        removed: dropped.chain(standing).collect(),
        perf: folded.perf.clone(),
        perf_removed: folded.perf_removals.clone(),
    }
}
