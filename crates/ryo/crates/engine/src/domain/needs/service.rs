//! The services of the `needs` concept: a site's formula walked once per file. A unit whose need does not hold is never
//! evaluated: a branch is not run, a table is not made. A decision depends on the file alone, never on what another rule
//! happened to build: a formula that asks a table makes it, in the formula's own order.

use super::types::{NeedFacts, NeedVerdict};
use crate::domain::code::records::service::need_at;
use crate::domain::code::records::types::NeedRecord;
use crate::domain::code::types::{NeedSite, PoolAt, ProgramCode, WordSlot};
use crate::domain::steps::types::MadeTableId;
use ryo_relations::domain::catalog::service::node_extent;
use ryo_relations::domain::relations::types::KindBits;

/// The slot `ahead` slots after a place among the needs.
fn slot(code: &ProgramCode, at: WordSlot, ahead: WordSlot) -> Option<WordSlot> {
    let place = at.0.checked_add(ahead.0)?;
    code.needs
        .get(usize::try_from(place).unwrap_or(usize::MAX))
        .copied()
}

/// The kinds of a list of the pool: its length, then its kinds.
fn kinds(code: &ProgramCode, list: PoolAt) -> &[WordSlot] {
    let from = usize::try_from(list.0).unwrap_or(usize::MAX);
    let length = code
        .pool
        .get(from)
        .map_or(0, |held| usize::try_from(held.0).unwrap_or(usize::MAX));
    code.pool
        .get(from.saturating_add(1)..from.saturating_add(1).saturating_add(length))
        .unwrap_or_default()
}

/// Whether every part of a record holds, or with `wanted` empty whether some part holds: the parts' places follow the
/// record's count. The walk stops at the first part that decides.
fn parts(
    facts: &mut impl NeedFacts,
    code: &ProgramCode,
    count: WordSlot,
    offsets: WordSlot,
    stops_at: NeedVerdict,
) -> NeedVerdict {
    for number in 0..count.0 {
        let part = slot(code, offsets, WordSlot(number))
            .map_or(NeedVerdict::Holds, |place| formula(facts, code, place));
        if part == stops_at {
            return stops_at;
        }
    }
    match stops_at {
        NeedVerdict::Holds => NeedVerdict::Empty,
        NeedVerdict::Empty => NeedVerdict::Holds,
    }
}

/// What the record at a place among the needs says of the file. A record that cannot be read says nothing is known: the
/// unit is evaluated.
fn formula(facts: &mut impl NeedFacts, code: &ProgramCode, at: WordSlot) -> NeedVerdict {
    let Some(record) = need_at(&code.needs, at) else {
        return NeedVerdict::Holds;
    };
    match record {
        NeedRecord::Unknown => NeedVerdict::Holds,
        NeedRecord::Kinds(list) => {
            let present = kinds(code, list).iter().any(|kind| {
                facts.kind_present(KindBits(u16::try_from(kind.0).unwrap_or(u16::MAX)))
                    == NeedVerdict::Holds
            });
            if present {
                NeedVerdict::Holds
            } else {
                NeedVerdict::Empty
            }
        }
        NeedRecord::Row {
            relation,
            column,
            constant,
        } => facts.relation_row(relation, column, constant),
        NeedRecord::Site(site) => decided(facts, code, site),
        NeedRecord::Table(table) => facts.table_row(table),
        NeedRecord::All {
            parts: count,
            offsets,
        } => parts(facts, code, count, offsets, NeedVerdict::Empty),
        NeedRecord::Any {
            parts: count,
            offsets,
        } => parts(facts, code, count, offsets, NeedVerdict::Holds),
    }
}

/// Adds to `out` every table the record at a place among the needs may ask for a row, whatever the file.
fn record_tables(
    code: &ProgramCode,
    at: WordSlot,
    seen: &mut hashbrown::HashSet<NeedSite>,
    out: &mut Vec<MadeTableId>,
) {
    match need_at(&code.needs, at) {
        Some(NeedRecord::Table(table)) => out.push(table),
        Some(NeedRecord::Site(site)) => tables_asked(code, site, seen, out),
        Some(
            NeedRecord::All {
                parts: count,
                offsets,
            }
            | NeedRecord::Any {
                parts: count,
                offsets,
            },
        ) => {
            for number in 0..count.0 {
                if let Some(place) = slot(code, offsets, WordSlot(number)) {
                    record_tables(code, place, seen, out);
                }
            }
        }
        Some(NeedRecord::Unknown | NeedRecord::Kinds(_) | NeedRecord::Row { .. }) | None => {}
    }
}

/// Adds to `out` every table the formula of `site` may ask for a row: the tables a decision can make, in any file. A
/// site in `seen` is not walked again: its tables were added when it was first met, so a walk over many units, whose
/// formulas name each other's sites again and again, reads each site's formula once.
pub(crate) fn tables_asked(
    code: &ProgramCode,
    site: NeedSite,
    seen: &mut hashbrown::HashSet<NeedSite>,
    out: &mut Vec<MadeTableId>,
) {
    if seen.insert(site)
        && let Some(place) = slot(code, WordSlot(site.0), WordSlot(0))
    {
        record_tables(code, place, seen, out);
    }
}

/// Whether the record at a place among the needs asks a relation of the node-table form for a row.
fn record_asks_nodes(code: &ProgramCode, at: WordSlot) -> NeedVerdict {
    match need_at(&code.needs, at) {
        Some(NeedRecord::Row { relation, .. }) => {
            let read = code
                .relations
                .get(usize::try_from(relation.0).unwrap_or(usize::MAX));
            if read.copied().and_then(node_extent).is_some() {
                NeedVerdict::Holds
            } else {
                NeedVerdict::Empty
            }
        }
        Some(NeedRecord::Site(site)) => asks_nodes(code, site),
        Some(
            NeedRecord::All {
                parts: count,
                offsets,
            }
            | NeedRecord::Any {
                parts: count,
                offsets,
            },
        ) => {
            let asked = (0..count.0).any(|number| {
                slot(code, offsets, WordSlot(number))
                    .is_some_and(|place| record_asks_nodes(code, place) == NeedVerdict::Holds)
            });
            if asked {
                NeedVerdict::Holds
            } else {
                NeedVerdict::Empty
            }
        }
        Some(NeedRecord::Unknown | NeedRecord::Kinds(_) | NeedRecord::Table(_)) | None => {
            NeedVerdict::Empty
        }
    }
}

/// Whether the formula of `site` asks a relation of the node-table form for a row (`Holds`), whatever the file: a rule that
/// consults such a decision read the node table.
pub(crate) fn asks_nodes(code: &ProgramCode, site: NeedSite) -> NeedVerdict {
    match slot(code, WordSlot(site.0), WordSlot(0)) {
        Some(place) => record_asks_nodes(code, place),
        None => NeedVerdict::Empty,
    }
}

/// Whether the unit that waits on `site` may have a row in the file: decided on the first ask, kept for the file.
pub(crate) fn decided(
    facts: &mut impl NeedFacts,
    code: &ProgramCode,
    site: NeedSite,
) -> NeedVerdict {
    if let Some(kept) = facts.kept(site) {
        return kept;
    }
    let verdict = match slot(code, WordSlot(site.0), WordSlot(0)) {
        Some(place) => formula(facts, code, place),
        None => NeedVerdict::Holds,
    };
    facts.keep(site, verdict);
    verdict
}
