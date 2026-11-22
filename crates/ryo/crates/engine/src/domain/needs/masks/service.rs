//! The services of the `masks` part: the terms of a rule set's needs, numbered as they are met; what a record of the
//! needs may cost, found once; and whether a site's formula asks the node rows, found once a site.

use super::types::{NeedMasks, NeedTerm, NodeAsk, RecordCost, TermNumber};
use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::records::service::need_at;
use crate::domain::code::records::types::NeedRecord;
use crate::domain::code::types::{NeedSite, ProgramCode, RelationNumber, WordSlot};
use ryo_relations::domain::catalog::service::node_extent;

/// The slot `ahead` slots after a place among the needs.
fn slot(code: &ProgramCode, at: WordSlot, ahead: WordSlot) -> Option<WordSlot> {
    let place = at.0.checked_add(ahead.0)?;
    code.needs
        .get(usize::try_from(place).unwrap_or(usize::MAX))
        .copied()
}

/// Whether a relation is of the node-table form: a row of it costs nothing to ask.
fn node_relation(code: &ProgramCode, relation: RelationNumber) -> bool {
    let read = code
        .relations
        .get(usize::try_from(relation.0).unwrap_or(usize::MAX));
    read.copied().and_then(node_extent).is_some()
}

/// The number of a term, given the first time it is met; the record at `at` is that term.
pub(crate) fn numbered(
    code: &ProgramCode,
    masks: &mut NeedMasks,
    term: NeedTerm,
    at: WordSlot,
) -> TermNumber {
    let NeedMasks {
        terms,
        term_costs,
        numbers,
        leaves,
        free_places,
        free_terms,
        ..
    } = masks;
    let number = *numbers.entry(term).or_insert_with(|| {
        let number = TermNumber(u32::try_from(terms.len()).unwrap_or(u32::MAX));
        terms.push(term);
        let cost = match term {
            NeedTerm::Kinds(_) | NeedTerm::Spelled { .. } | NeedTerm::Path { .. } => RecordCost::Free,
            NeedTerm::Row { relation, .. } if node_relation(code, relation) => RecordCost::Free,
            NeedTerm::Row {
                column, constant, ..
            } if column == NO_SLOT || constant == NO_SLOT => RecordCost::Presence,
            NeedTerm::Row { .. } => RecordCost::Dear,
        };
        term_costs.push(cost);
        if cost == RecordCost::Free {
            free_terms.push(number);
            free_places.push(u32::try_from(free_terms.len()).unwrap_or(0));
        } else {
            free_places.push(0);
        }
        number
    });
    // A term made from another (no record of its own: `NO_SLOT`) is no leaf of the walk.
    if at == NO_SLOT {
        return number;
    }
    let place = usize::try_from(at.0).unwrap_or(usize::MAX);
    if leaves.len() <= place {
        leaves.resize(place.saturating_add(1), 0);
    }
    if let Some(leaf) = leaves.get_mut(place) {
        *leaf = number.0.saturating_add(1);
    }
    number
}

/// Whether the record at a place may cost a fill, whatever the file: found once a place.
fn record_cost(code: &ProgramCode, at: WordSlot, costs: &mut Vec<RecordCost>) -> RecordCost {
    let place = usize::try_from(at.0).unwrap_or(usize::MAX);
    match costs.get(place) {
        Some(RecordCost::Unknown) => {}
        Some(known) => return *known,
        None => return RecordCost::Dear,
    }
    // A formula that names itself is taken as dear while it is found.
    if let Some(held) = costs.get_mut(place) {
        *held = RecordCost::Dear;
    }
    let dear = match need_at(&code.needs, at) {
        Some(NeedRecord::Unknown | NeedRecord::Kinds(_) | NeedRecord::Path { .. }) | None => false,
        Some(NeedRecord::Row { relation, .. }) => !node_relation(code, relation),
        Some(NeedRecord::Site(site)) => slot(code, WordSlot(site.0), WordSlot(0))
            .is_some_and(|formula| record_cost(code, formula, costs) == RecordCost::Dear),
        Some(NeedRecord::Table(_)) => true,
        Some(
            NeedRecord::All {
                parts: count,
                offsets,
            }
            | NeedRecord::Any {
                parts: count,
                offsets,
            },
        ) => (0..count.0).any(|number| {
            slot(code, offsets, WordSlot(number))
                .is_some_and(|part| record_cost(code, part, costs) == RecordCost::Dear)
        }),
    };
    let cost = if dear {
        RecordCost::Dear
    } else {
        RecordCost::Free
    };
    if let Some(held) = costs.get_mut(place) {
        *held = cost;
    }
    cost
}

/// Finds, for every place among the needs, whether its record may cost a fill: once for a worker.
pub(crate) fn costs_found(code: &ProgramCode, masks: &mut NeedMasks) {
    masks.costs.clear();
    masks.costs.resize(code.needs.len(), RecordCost::Unknown);
    for place in 0..code.needs.len() {
        // Only a place that starts a record is asked by the walk; any other is found dear or free and never read.
        let at = WordSlot(u32::try_from(place).unwrap_or(u32::MAX));
        let _cost = record_cost(code, at, &mut masks.costs);
    }
}

/// Whether the record at a place among the needs asks a relation of the node-table form for a row.
fn record_asks_nodes(code: &ProgramCode, at: WordSlot, masks: &mut NeedMasks) -> NodeAsk {
    match need_at(&code.needs, at) {
        Some(NeedRecord::Row { relation, .. }) if node_relation(code, relation) => NodeAsk::Asked,
        Some(NeedRecord::Site(site)) => node_asked(code, site, masks),
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
                if let Some(place) = slot(code, offsets, WordSlot(number))
                    && record_asks_nodes(code, place, masks) == NodeAsk::Asked
                {
                    return NodeAsk::Asked;
                }
            }
            NodeAsk::NotAsked
        }
        Some(
            NeedRecord::Unknown
            | NeedRecord::Kinds(_)
            | NeedRecord::Table(_)
            | NeedRecord::Row { .. }
            | NeedRecord::Path { .. },
        )
        | None => NodeAsk::NotAsked,
    }
}

/// Whether the formula of `site` asks a relation of the node-table form for a row, whatever the file: a rule that
/// consults such a decision read the node table. Found once a site and kept: a site many formulas name is walked once
/// (`needs::service::asks_nodes` walked it once for each formula that names it, at every level).
pub(crate) fn node_asked(code: &ProgramCode, site: NeedSite, masks: &mut NeedMasks) -> NodeAsk {
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    match masks.node_asks.get(place).copied() {
        Some(NodeAsk::Asked) => return NodeAsk::Asked,
        Some(NodeAsk::NotAsked | NodeAsk::Finding) => return NodeAsk::NotAsked,
        Some(NodeAsk::Unknown) | None => {}
    }
    if masks.node_asks.len() <= place {
        masks
            .node_asks
            .resize(place.saturating_add(1), NodeAsk::Unknown);
    }
    if let Some(kept) = masks.node_asks.get_mut(place) {
        *kept = NodeAsk::Finding;
    }
    let found = match slot(code, WordSlot(site.0), WordSlot(0)) {
        Some(formula) => record_asks_nodes(code, formula, masks),
        None => NodeAsk::NotAsked,
    };
    if let Some(kept) = masks.node_asks.get_mut(place) {
        *kept = found;
    }
    found
}
