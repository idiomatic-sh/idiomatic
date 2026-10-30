//! The `ratchet` service: the records `up` and `down` write, with no IO and no ports.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{Accepting, DropReason, Dropped, Lowered, Lowering, Moved};
use crate::domain::judgement::service::lowered;
use crate::domain::judgement::types::{Judgement, Resolved};
use crate::domain::operations::types::{Count, Day, Provenance, Reason, Record};
use crate::domain::renames::service::changes;
use crate::domain::renames::types::Carried;
use crate::domain::violations::service::key;
use crate::domain::violations::types::{Id, Key, Keyed, Standing, Violation};

/// The records accepting `new` with `reason` on `since` at `revision`: one per key, its count what the lock already
/// accepts under it plus what is new, so the operation's record supersedes the one before. A suppressed finding takes
/// no reason: its reason is the in-source ignore beside its code; `down` records only those, so it gives none.
#[must_use]
pub fn accepting(
    existing: &[Record],
    new: &[Violation],
    revision: &Provenance,
    since: &Day,
    reason: Option<&Reason>,
    id_of: &dyn Fn(&Key) -> Id,
) -> Accepting {
    let before: BTreeMap<&Id, Count> = existing
        .iter()
        .map(|record| (&record.id, record.count))
        .collect();
    let mut records: BTreeMap<Id, Record> = BTreeMap::new();
    let mut unkeyed = 0_u32;
    for violation in new {
        let Keyed::Key(key) = key(violation) else {
            unkeyed += 1;
            continue;
        };
        let id = id_of(&key);
        let _: &mut Record = records
            .entry(id.clone())
            .and_modify(|record| record.count.0 += 1)
            .or_insert_with(|| Record {
                count: Count(before.get(&id).map_or(0, |count| count.0) + 1),
                id,
                reasons: match key.standing {
                    Standing::Reported => reason.into_iter().cloned().collect(),
                    Standing::Suppressed => Vec::new(),
                },
                key,
                message: violation.message.clone(),
                since_rev: revision.clone(),
                since: since.clone(),
            });
    }
    Accepting {
        records: records.into_values().collect(),
        unkeyed: Count(unkeyed),
    }
}

/// What `down` drops and lowers: each resolved record, lowered to what still occurs or dropped, and each projection
/// the run cannot carry, dropped from its old path. What of a record the run reports at another path is carried there
/// (`relocate`), so the record keeps at its own path only what is reported there.
#[must_use]
pub fn lowering(outcome: &Judgement, carried: &Carried) -> Lowering {
    let renamed: BTreeSet<&Id> = carried
        .unmatched
        .iter()
        .map(|migration| &migration.projected.id)
        .collect();
    let mut moved: BTreeMap<&Id, (&Record, u32)> = BTreeMap::new();
    for relocated in &outcome.relocated {
        moved
            .entry(&relocated.record.id)
            .or_insert((&relocated.record, 0))
            .1 += relocated.count.0;
    }
    let mut lowered_records = Vec::new();
    let mut gone: Vec<Resolved> = Vec::new();
    let mut kept = Vec::new();
    for resolved in outcome
        .resolved
        .iter()
        .filter(|resolved| !renamed.contains(&resolved.record.id))
    {
        let away = moved
            .remove(&resolved.record.id)
            .map_or(0, |(_, count)| count);
        let here = Resolved {
            remaining: Count(resolved.remaining.0.saturating_sub(away)),
            ..resolved.clone()
        };
        match lowered(&here) {
            Some(record) => lowered_records.push(Lowered {
                record,
                from: Count(resolved.count.0 + resolved.remaining.0),
            }),
            None => gone.push(here),
        }
    }
    for (record, away) in moved.into_values() {
        let here = record.count.0.saturating_sub(away);
        if here > 0 {
            kept.push(Record {
                count: Count(here),
                ..record.clone()
            });
        }
    }
    let mut dropped: Vec<Dropped> = carried
        .unmatched
        .iter()
        .map(|migration| Dropped {
            record: migration.original.clone(),
            moved: Moved::To(migration.projected.key.path.clone()),
            reason: DropReason::RenamedWithoutFinding,
        })
        .collect();
    dropped.extend(gone.iter().map(|resolved| Dropped {
        record: resolved.record.clone(),
        moved: Moved::Stayed,
        reason: DropReason::Resolved(resolved.resolution),
    }));
    let changes = changes(
        carried,
        lowered_records
            .iter()
            .map(|lowered| lowered.record.clone())
            .chain(kept)
            .collect(),
        gone.iter()
            .map(|resolved| resolved.record.clone())
            .collect(),
    );
    Lowering {
        dropped,
        lowered: lowered_records,
        changes,
    }
}
