//! The `renames` service: which renames apply to which record, and the records a ratchet persists with them, with no
//! IO and no ports.

use std::collections::{BTreeMap, BTreeSet};

use super::constants::{EXTENSION, EXTENSION_MARK, ID_DIGITS};
use super::types::{
    Basis, Carried, Changes, Head, HistoryStart, Introduction, Migration, Parent, Projection,
    Rename, RenameEvent,
};
use crate::domain::judgement::types::Judgement;
use crate::domain::operations::types::{Count, OperationId, Provenance, Record, Removal, Revision};
use crate::domain::violations::types::{Id, Key, RepoPath};

/// The operation a lock file's path names, when its stem is a lowercase BLAKE3 digest.
#[must_use]
pub fn operation_of(path: &RepoPath) -> Option<OperationId> {
    let name = path.0.rsplit('/').next()?;
    let (stem, extension) = name.rsplit_once(EXTENSION_MARK)?;
    if extension != EXTENSION {
        return None;
    }
    let digest = stem.len() == ID_DIGITS
        && stem
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    digest.then(|| OperationId(stem.into()))
}

/// The first commit that added each operation, in history order.
#[must_use]
pub fn introduced_by(introductions: &[Introduction]) -> BTreeMap<OperationId, Revision> {
    let mut by = BTreeMap::new();
    for introduction in introductions {
        if let Some(id) = operation_of(&introduction.path) {
            let _: &mut Revision = by
                .entry(id)
                .or_insert_with(|| introduction.revision.clone());
        }
    }
    by
}

/// The earliest introduction of an operation that accepted any record: after its parent starts the rename history
/// that can apply to any of them. None when no introduction is known.
#[must_use]
pub fn earliest<'a>(
    introductions: &'a [Introduction],
    origins: &BTreeMap<Id, Vec<OperationId>>,
    introduced: &BTreeMap<OperationId, Revision>,
) -> Option<&'a Introduction> {
    introductions.iter().find(|introduction| {
        origins
            .values()
            .flatten()
            .any(|origin| introduced.get(origin) == Some(&introduction.revision))
    })
}

/// Where the rename history that can apply to any of the records `origins` names starts: after the parent of the
/// earliest commit that added one of their operations, or at the first commit when that is the root. None when no
/// such commit is known: only the working tree's renames apply.
#[must_use]
pub fn history_start(
    introductions: &[Introduction],
    origins: &BTreeMap<Id, Vec<OperationId>>,
    introduced: &BTreeMap<OperationId, Revision>,
) -> Option<HistoryStart> {
    earliest(introductions, origins, introduced).map(|first| match &first.parent {
        Parent::Commit(parent) => HistoryStart::After(parent.clone()),
        Parent::Root => HistoryStart::Beginning,
    })
}

/// Carries each record through the committed renames its basis says apply to it, then the working tree's; a moved
/// record is keyed at its new path, its id derived by `id_of`, and accepted at `head`.
#[must_use]
pub fn project(
    records: Vec<Record>,
    head: &Revision,
    history: &[RenameEvent],
    worktree: &[Rename],
    basis: Basis<'_>,
    id_of: &dyn Fn(&Key) -> Id,
) -> Projection {
    let positions: BTreeMap<&Revision, usize> = history
        .iter()
        .enumerate()
        .map(|(index, event)| (&event.revision, index))
        .collect();
    let committed: Vec<(usize, &Rename)> = history
        .iter()
        .enumerate()
        .flat_map(|(index, event)| event.renames.iter().map(move |rename| (index, rename)))
        .collect();
    let mut projected = Vec::with_capacity(records.len());
    let mut migrations = Vec::new();
    for original in records {
        let start = match basis {
            Basis::Operations {
                origins,
                introduced,
            } => origins.get(&original.id).and_then(|ids| {
                ids.iter()
                    .filter_map(|id| introduced.get(id))
                    .filter_map(|revision| positions.get(revision))
                    .min()
                    .copied()
            }),
            Basis::Branch => Some(0),
            Basis::Unknown => None,
        };
        let mut path = original.key.path.clone();
        let applying = committed
            .iter()
            .filter(|(index, _)| start.is_some_and(|start| *index >= start))
            .map(|(_, rename)| *rename)
            .chain(worktree);
        for rename in applying {
            if path == rename.from {
                path.clone_from(&rename.to);
            }
        }
        if path == original.key.path {
            projected.push(original);
            continue;
        }
        let mut moved = original.clone();
        moved.key.path = path;
        moved.id = id_of(&moved.key);
        moved.since_rev = Provenance::At(head.clone());
        projected.push(moved.clone());
        migrations.push(Migration {
            original,
            projected: moved,
        });
    }
    Projection {
        records: projected,
        migrations,
    }
}

/// The accepted set as another branch holds it: the union of its lock at the merge base and at its tip. The base
/// alone would blame a branch for what the target accepted after it forked; the tip alone for what it fixed since.
#[must_use]
pub fn on_branch(base: Vec<Record>, target: Vec<Record>) -> Vec<Record> {
    let mut accepted: BTreeMap<Id, Record> = base
        .into_iter()
        .map(|record| (record.id.clone(), record))
        .collect();
    for candidate in target {
        let _: &mut Record = accepted
            .entry(candidate.id.clone())
            .and_modify(|record| {
                record.count = record.count.max(candidate.count);
                record.reasons.extend(candidate.reasons.iter().cloned());
                record.reasons.sort();
                record.reasons.dedup();
            })
            .or_insert(candidate);
    }
    accepted.into_values().collect()
}

/// Splits the projections by whether the run reports their identity: `observed`, every key it reported, holds the
/// projected one, or the comparison matched the projection to a violation at another path, its code moved again, so
/// it is neither wholly resolved nor unjudged in `outcome`.
#[must_use]
pub fn split(migrations: &[Migration], observed: &BTreeSet<Key>, outcome: &Judgement) -> Carried {
    let unused: BTreeSet<&Id> = outcome
        .resolved
        .iter()
        .filter(|resolved| resolved.remaining.0 == 0)
        .map(|resolved| &resolved.record.id)
        .chain(outcome.stale.iter().map(|stale| &stale.record.id))
        .collect();
    let (carried, unmatched) = migrations.iter().cloned().partition(|migration| {
        observed.contains(&migration.projected.key) || !unused.contains(&migration.projected.id)
    });
    Carried { carried, unmatched }
}

/// Carries each record whose code the run reports at another path, with no rename git records, to that path: a
/// projection keyed where the violations are reported now, its id derived by `id_of`, accepted at `head`, counting
/// what moved there and what the lock's `records` already accept under that key. A record that is itself a rename
/// projection is carried from the path the lock holds it at, in place of the rename. What stays at the old path is
/// `down`'s to lower.
#[must_use]
pub fn relocate(
    carried: Carried,
    outcome: &Judgement,
    records: &[Record],
    head: &Head,
    id_of: &dyn Fn(&Key) -> Id,
) -> Carried {
    if outcome.relocated.is_empty() {
        return carried;
    }
    let Carried {
        carried: renames,
        unmatched,
    } = carried;
    let originals: BTreeMap<&Id, &Record> = renames
        .iter()
        .map(|migration| (&migration.projected.id, &migration.original))
        .collect();
    let mut targets: BTreeMap<Id, Record> = BTreeMap::new();
    let mut moves: Vec<(Record, Id)> = Vec::new();
    for relocated in &outcome.relocated {
        let id = id_of(&relocated.to);
        let target = targets.entry(id.clone()).or_insert_with(|| {
            records
                .iter()
                .find(|record| record.id == id)
                .cloned()
                .unwrap_or_else(|| Record {
                    id: id.clone(),
                    key: relocated.to.clone(),
                    count: Count(0),
                    since_rev: match head {
                        Head::Commit(revision) => Provenance::At(revision.clone()),
                        Head::Detached => relocated.record.since_rev.clone(),
                    },
                    ..relocated.record.clone()
                })
        });
        target.count.0 += relocated.count.0;
        let original = originals
            .get(&relocated.record.id)
            .map_or_else(|| relocated.record.clone(), |original| (*original).clone());
        moves.push((original, id));
    }
    let moving: BTreeSet<&Id> = outcome
        .relocated
        .iter()
        .map(|relocated| &relocated.record.id)
        .collect();
    let mut kept: Vec<Migration> = renames
        .iter()
        .filter(|migration| !moving.contains(&migration.projected.id))
        .cloned()
        .collect();
    kept.extend(moves.into_iter().filter_map(|(original, id)| {
        targets.get(&id).map(|projected| Migration {
            original,
            projected: projected.clone(),
        })
    }));
    Carried {
        carried: kept,
        unmatched,
    }
}

/// What an operation persists with `added` and `dropped`: a carried projection accepted at its new path and removed
/// from its old one, an unmatched one removed from its old path.
#[must_use]
pub fn changes(carried: &Carried, added: Vec<Record>, dropped: Vec<Record>) -> Changes {
    let dropped_ids: BTreeSet<Id> = dropped.iter().map(|record| record.id.clone()).collect();
    let migrated: BTreeSet<Id> = carried
        .carried
        .iter()
        .map(|migration| migration.projected.id.clone())
        .collect();
    let mut additions: BTreeMap<Id, Record> = carried
        .carried
        .iter()
        .filter(|migration| !dropped_ids.contains(&migration.projected.id))
        .map(|migration| (migration.projected.id.clone(), migration.projected.clone()))
        .collect();
    additions.extend(added.into_iter().map(|record| (record.id.clone(), record)));
    let mut removals: BTreeMap<Id, Record> = carried
        .carried
        .iter()
        .chain(&carried.unmatched)
        .map(|migration| (migration.original.id.clone(), migration.original.clone()))
        .collect();
    removals.extend(
        dropped
            .into_iter()
            .filter(|record| !migrated.contains(&record.id))
            .map(|record| (record.id.clone(), record)),
    );
    removals.retain(|id, _| !additions.contains_key(id));
    Changes {
        added: additions.into_values().collect(),
        removed: removals
            .into_values()
            .map(|record| Removal {
                id: record.id,
                key: record.key,
            })
            .collect(),
    }
}
